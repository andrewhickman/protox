use std::collections::{hash_map, HashMap};

use miette::SourceSpan;
use prost_types::field_descriptor_proto;

use crate::{
    ast::Syntax,
    case::to_lower_without_underscores,
    index_to_i32,
    lines::LineResolver,
    resolve_span, tag,
    types::{
        source_code_info::Location, DescriptorProto, FieldDescriptorProto, FileDescriptorProto,
        MethodDescriptorProto, ServiceDescriptorProto,
    },
};

use super::{names::DefinitionKind, CheckError, NameMap};

/// Resolve and check relative type names and options.
pub(crate) fn resolve(
    file: &mut FileDescriptorProto,
    lines: Option<&LineResolver>,
    name_map: &NameMap,
) -> Result<(), Vec<CheckError>> {
    let source_code_info = file.source_code_info.take();
    let locations = source_code_info
        .as_ref()
        .map(|s| s.location.as_slice())
        .unwrap_or(&[]);
    let syntax = match file.syntax() {
        "" | "proto2" => Syntax::Proto2,
        "proto3" => Syntax::Proto3,
        syntax => {
            return Err(vec![CheckError::UnknownSyntax {
                syntax: syntax.to_owned(),
                span: resolve_span(lines, locations, &[tag::file::SYNTAX]).map(SourceSpan::from),
            }])
        }
    };

    let mut context = Context {
        syntax,
        name_map,
        scope: String::new(),
        errors: Vec::new(),
        path: Vec::new(),
        locations,
        lines,
        is_google_descriptor: file.name() == "google/protobuf/descriptor.proto",
    };

    context.resolve_file(file);
    debug_assert!(context.scope.is_empty());

    let errors = context.errors;
    file.source_code_info = source_code_info;

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

struct Context<'a> {
    syntax: Syntax,
    name_map: &'a NameMap,
    scope: String,
    path: Vec<i32>,
    locations: &'a [Location],
    lines: Option<&'a LineResolver>,
    errors: Vec<CheckError>,
    is_google_descriptor: bool,
}

impl<'a> Context<'a> {
    fn resolve_file(&mut self, file: &mut FileDescriptorProto) {
        if !file.package().is_empty() {
            for part in file.package().split('.') {
                self.enter_scope(part);
            }
        }

        self.path.extend(&[tag::file::MESSAGE_TYPE, 0]);
        for message in &mut file.message_type {
            self.resolve_descriptor_proto(message);
            self.bump_path();
        }

        // self.replace_path(&[tag::file::ENUM_TYPE, 0]);
        // for enu in &file.enum_type {
        //     self.add_enum_descriptor_proto(enu);
        //     self.bump_path();
        // }

        self.replace_path(&[tag::file::EXTENSION, 0]);
        for extend in &mut file.extension {
            self.resolve_field_descriptor_proto(extend);
            self.bump_path();
        }

        self.replace_path(&[tag::file::SERVICE, 0]);
        for service in &mut file.service {
            self.resolve_service_descriptor_proto(service);
            self.bump_path();
        }
        self.pop_path(2);

        if !file.package().is_empty() {
            for _ in file.package().split('.') {
                self.exit_scope();
            }
        }
    }

    fn resolve_descriptor_proto(&mut self, message: &mut DescriptorProto) {
        self.enter_scope(message.name());
        self.path.extend(&[tag::message::FIELD, 0]);
        for field in &mut message.field {
            self.resolve_field_descriptor_proto(field);
            self.bump_path();
        }
        self.pop_path(2);
        self.exit_scope();

        if self.syntax != Syntax::Proto2 {
            self.path.push(tag::message::FIELD);
            self.check_message_field_camel_case_names(message.field.iter());
            self.path.pop();
        }
    }

    fn resolve_field_descriptor_proto(&mut self, field: &mut FieldDescriptorProto) {
        if let Some(def) = self.resolve_type_name(&mut field.extendee, &[tag::field::EXTENDEE]) {
            if !matches!(def, DefinitionKind::Message) {
                let span = self.resolve_span(&[tag::field::EXTENDEE]);
                self.errors.push(CheckError::InvalidExtendeeTypeName {
                    name: field.extendee().to_owned(),
                    span,
                });
            }
        }

        if let Some(def) = self.resolve_type_name(&mut field.type_name, &[tag::field::TYPE_NAME]) {
            match def {
                DefinitionKind::Message => {
                    if field.r#type != Some(field_descriptor_proto::Type::Group as _) {
                        field.r#type = Some(field_descriptor_proto::Type::Message as _);
                    }
                }
                DefinitionKind::Enum => {
                    field.r#type = Some(field_descriptor_proto::Type::Enum as _);
                }
                _ => {
                    let span = self.resolve_span(&[tag::field::TYPE_NAME]);
                    self.errors.push(CheckError::InvalidMessageFieldTypeName {
                        name: field.type_name().to_owned(),
                        span,
                    });
                }
            }
        }
    }

    fn check_message_field_camel_case_names<'b>(
        &mut self,
        fields: impl Iterator<Item = &'b FieldDescriptorProto>,
    ) {
        let mut names: HashMap<String, (&'b str, i32)> = HashMap::new();
        for (index, field) in fields.enumerate() {
            let name = field.name();
            let index = index_to_i32(index);

            match names.entry(to_lower_without_underscores(name)) {
                hash_map::Entry::Occupied(entry) => {
                    let first = self.resolve_span(&[entry.get().1, tag::field::NAME]);
                    let second = self.resolve_span(&[index, tag::field::NAME]);

                    self.errors.push(CheckError::DuplicateCamelCaseFieldName {
                        first_name: entry.get().0.to_owned(),
                        first,
                        second_name: name.to_owned(),
                        second,
                    })
                }
                hash_map::Entry::Vacant(entry) => {
                    entry.insert((name, index));
                }
            }
        }
    }

    fn resolve_service_descriptor_proto(&mut self, service: &mut ServiceDescriptorProto) {
        self.enter_scope(service.name());
        self.path.extend(&[tag::service::METHOD, 0]);
        for method in &mut service.method {
            self.resolve_method_descriptor_proto(method);
            self.bump_path();
        }
        self.pop_path(2);
        self.exit_scope();
    }

    fn resolve_method_descriptor_proto(&mut self, method: &mut MethodDescriptorProto) {
        let input_ty = self.resolve_type_name(&mut method.name, &[tag::method::INPUT_TYPE]);
        if !matches!(input_ty, None | Some(DefinitionKind::Message)) {
            let span = self.resolve_span(&[tag::method::INPUT_TYPE]);
            self.errors.push(CheckError::InvalidMethodTypeName {
                name: method.input_type().to_owned(),
                kind: "input",
                span,
            })
        }

        let output_ty = self.resolve_type_name(&mut method.name, &[tag::method::OUTPUT_TYPE]);
        if !matches!(output_ty, None | Some(DefinitionKind::Message)) {
            let span = self.resolve_span(&[tag::method::INPUT_TYPE]);
            self.errors.push(CheckError::InvalidMethodTypeName {
                name: method.output_type().to_owned(),
                kind: "output",
                span,
            })
        }
    }

    /*
    fn check_option(
        &mut self,
        mut result: &mut OptionSet,
        namespace: &str,
        option: &ast::OptionBody,
        option_span: Span,
    ) -> Result<(), ()> {
        use field_descriptor_proto::{Label, Type};

        // Special cases for things which use option syntax but aren't options
        if namespace == "google.protobuf.FieldOptions" && option.is("default") {
            return Ok(());
        }

        if self.name_map.is_none() && !option.is_simple() {
            todo!("set uninterpreted_option")
        }

        let mut numbers = vec![];
        let mut ty = None;
        let mut is_repeated = false;
        let mut type_name = Some(Cow::Borrowed(namespace));
        let mut type_name_context = None;

        for part in &option.name {
            let namespace = match (ty, &type_name) {
                (None | Some(Type::Message | Type::Group), Some(name)) => name.as_ref(),
                _ => {
                    self.errors.push(CheckError::OptionScalarFieldAccess {
                        span: option.name_span(),
                    });
                    return Err(());
                }
            };

            if let Some(&number) = numbers.last() {
                result = result.get_message_mut(number, part.span());
            }

            match part {
                ast::OptionNamePart::Ident(name) => {
                    let full_name = make_name(namespace, name);
                    if let Some(DefinitionKind::Field {
                        number: field_number,
                        label,
                        ty: field_ty,
                        type_name: field_type_name,
                        extendee: None,
                        ..
                    }) = self.get_option_def(&full_name)
                    {
                        numbers.push(*field_number);
                        ty = *field_ty;
                        is_repeated = matches!(label, Some(Label::Repeated));
                        type_name_context = Some(namespace.to_owned());
                        type_name = field_type_name.clone().map(Cow::Owned);
                    } else {
                        self.errors.push(CheckError::OptionUnknownField {
                            name: name.to_string(),
                            namespace: namespace.to_owned(),
                            span: name.span.clone(),
                        });
                        return Err(());
                    }
                }
                ast::OptionNamePart::Extension(_, _) => todo!(),
            }
        }

        let value = self.check_option_value(
            ty,
            option,
            type_name_context.as_deref().unwrap_or_default(),
            type_name.as_deref(),
        )?;

        self.add_location_for(&numbers, option_span);
        if is_repeated {
            result.set_repeated(
                *numbers.last().expect("expected at least one field access"),
                value,
                option.name_span(),
            )
        } else if let Err(first) = result.set(
            *numbers.last().expect("expected at least one field access"),
            value,
            option.name_span(),
        ) {
            self.errors.push(CheckError::OptionAlreadySet {
                name: option.name_string(),
                first,
                second: option.name_span(),
            })
        }

        Ok(())
    }

    fn check_option_value(
        &mut self,
        ty: Option<field_descriptor_proto::Type>,
        option: &ast::OptionBody,
        type_name_context: &str,
        type_name: Option<&str>,
    ) -> Result<options::Value, ()> {
        use field_descriptor_proto::Type;

        Ok(match ty {
            Some(Type::Double) => {
                options::Value::Double(self.check_option_value_f64(&option.value)?)
            }
            Some(Type::Float) => {
                options::Value::Float(self.check_option_value_f64(&option.value)? as f32)
            }
            Some(Type::Int32) => options::Value::Int32(self.check_option_value_i32(&option.value)?),
            Some(Type::Int64) => options::Value::Int64(self.check_option_value_i64(&option.value)?),
            Some(Type::Uint32) => {
                options::Value::Uint32(self.check_option_value_u32(&option.value)?)
            }
            Some(Type::Uint64) => {
                options::Value::Uint64(self.check_option_value_u64(&option.value)?)
            }
            Some(Type::Sint32) => {
                options::Value::Sint32(self.check_option_value_i32(&option.value)?)
            }
            Some(Type::Sint64) => {
                options::Value::Sint64(self.check_option_value_i64(&option.value)?)
            }
            Some(Type::Fixed32) => {
                options::Value::Fixed32(self.check_option_value_u32(&option.value)?)
            }
            Some(Type::Fixed64) => {
                options::Value::Fixed64(self.check_option_value_u64(&option.value)?)
            }
            Some(Type::Sfixed32) => {
                options::Value::Sfixed32(self.check_option_value_i32(&option.value)?)
            }
            Some(Type::Sfixed64) => {
                options::Value::Sfixed64(self.check_option_value_i64(&option.value)?)
            }
            Some(Type::Bool) => options::Value::Bool(self.check_option_value_bool(&option.value)?),
            Some(Type::String) => {
                options::Value::String(self.check_option_value_string(&option.value)?)
            }
            Some(Type::Bytes) => {
                options::Value::Bytes(self.check_option_value_bytes(&option.value)?)
            }
            Some(Type::Enum) => {
                let value = self.check_option_value_enum(
                    &option.value,
                    type_name_context,
                    type_name.unwrap_or_default(),
                )?;
                options::Value::Int32(value)
            }
            None | Some(Type::Message | Type::Group) => {
                let type_name = type_name.unwrap_or_default();
                match self.resolve_option_def(type_name_context, type_name) {
                    Some(DefinitionKind::Message) => {
                        let value = self.check_option_value_message(&option.value, type_name)?;
                        if ty == Some(Type::Group) {
                            options::Value::Group(value)
                        } else {
                            options::Value::Message(value)
                        }
                    }
                    Some(DefinitionKind::Enum) => {
                        let value = self.check_option_value_enum(
                            &option.value,
                            type_name_context,
                            type_name,
                        )?;
                        options::Value::Int32(value)
                    }
                    Some(_) | None => {
                        self.errors.push(CheckError::OptionInvalidTypeName {
                            name: type_name.to_owned(),
                            span: option.name_span(),
                        });
                        return Err(());
                    }
                }
            }
        })
    }

    fn check_option_value_f64(&mut self, value: &ast::OptionValue) -> Result<f64, ()> {
        match value {
            ast::OptionValue::Float(float) => Ok(float.value),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "a float".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_i32(&mut self, value: &ast::OptionValue) -> Result<i32, ()> {
        match self.check_option_value_int(value)?.as_i32() {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "a signed 32-bit integer".to_owned(),
                    actual: value.to_string(),
                    min: i32::MIN.to_string(),
                    max: i32::MAX.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_i64(&mut self, value: &ast::OptionValue) -> Result<i64, ()> {
        match self.check_option_value_int(value)?.as_i64() {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "a signed 64-bit integer".to_owned(),
                    actual: value.to_string(),
                    min: i64::MIN.to_string(),
                    max: i64::MAX.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_u32(&mut self, value: &ast::OptionValue) -> Result<u32, ()> {
        match self.check_option_value_int(value)?.as_u32() {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "an unsigned 32-bit integer".to_owned(),
                    actual: value.to_string(),
                    min: u32::MIN.to_string(),
                    max: u32::MAX.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_u64(&mut self, value: &ast::OptionValue) -> Result<u64, ()> {
        match self.check_option_value_int(value)?.as_u64() {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "an unsigned 64-bit integer".to_owned(),
                    actual: value.to_string(),
                    min: u64::MIN.to_string(),
                    max: u64::MAX.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_int<'b>(
        &mut self,
        value: &'b ast::OptionValue,
    ) -> Result<&'b ast::Int, ()> {
        match value {
            ast::OptionValue::Int(int) => Ok(int),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "an integer".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_bool(&mut self, value: &ast::OptionValue) -> Result<bool, ()> {
        match value {
            ast::OptionValue::Ident(ident) if ident.value.as_str() == "false" => Ok(false),
            ast::OptionValue::Ident(ident) if ident.value.as_str() == "true" => Ok(true),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "either 'true' or 'false'".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_string(&mut self, value: &ast::OptionValue) -> Result<String, ()> {
        let bytes = self.check_option_value_bytes(value)?;
        match String::from_utf8(bytes) {
            Ok(string) => Ok(string),
            Err(_) => {
                self.errors
                    .push(CheckError::StringValueInvalidUtf8 { span: value.span() });
                Err(())
            }
        }
    }

    fn check_option_value_bytes(&mut self, value: &ast::OptionValue) -> Result<Vec<u8>, ()> {
        match value {
            ast::OptionValue::String(string) => Ok(string.value.clone()),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "a string".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn check_option_value_message(
        &mut self,
        _value: &ast::OptionValue,
        _type_name: &str,
    ) -> Result<OptionSet, ()> {
        todo!()
    }

    fn check_option_value_enum(
        &mut self,
        value: &ast::OptionValue,
        context: &str,
        type_name: &str,
    ) -> Result<i32, ()> {
        let type_namespace = parse_namespace(type_name);

        match value {
            ast::OptionValue::Ident(ident) => {
                match self.resolve_option_def(context, &make_name(type_namespace, &ident.value)) {
                    Some(DefinitionKind::EnumValue { number }) => Ok(*number),
                    _ => {
                        self.errors.push(CheckError::InvalidEnumValue {
                            value_name: ident.value.clone(),
                            enum_name: type_name.to_owned(),
                            span: ident.span.clone(),
                        });
                        Err(())
                    }
                }
            }
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "an enum value identifier".to_owned(),
                    actual: value.to_string(),
                    span: value.span(),
                });
                Err(())
            }
        }
    }

    fn get_option_def(&self, name: &str) -> Option<&DefinitionKind> {
        if let Some(name_map) = &self.name_map {
            if let Some(def) = name_map.get(name) {
                return Some(def);
            }
        }

        if !self.is_google_descriptor {
            return NameMap::google_descriptor().get(name);
        }

        None
    }

    fn resolve_option_def(&self, context: &str, name: &str) -> Option<&DefinitionKind> {
        if let Some(name_map) = &self.name_map {
            if let Some((_, def)) = name_map.resolve(context, name) {
                return Some(def);
            }
        }

        if !self.is_google_descriptor {
            if let Some((_, def)) = NameMap::google_descriptor().resolve(context, name) {
                return Some(def);
            }
        }

        None
    }
    */

    fn resolve_type_name(
        &mut self,
        name: &mut Option<String>,
        path_items: &[i32],
    ) -> Option<&DefinitionKind> {
        if let Some(name) = name.as_mut() {
            if let Some((resolved_name, def)) = self.name_map.resolve(self.scope_name(), name) {
                *name = resolved_name.into_owned();
                Some(def)
            } else {
                let span = self.resolve_span(path_items);
                self.errors.push(CheckError::TypeNameNotFound {
                    name: name.to_owned(),
                    span,
                });
                None
            }
        } else {
            None
        }
    }

    fn enter_scope(&mut self, name: &str) {
        if !self.scope.is_empty() {
            self.scope.push('.');
        }
        self.scope.push_str(name);
    }

    fn exit_scope(&mut self) {
        let len = self.scope.rfind('.').unwrap_or(0);
        self.scope.truncate(len);
    }

    fn scope_name(&self) -> &str {
        &self.scope
    }

    fn resolve_span(&mut self, path_items: &[i32]) -> Option<SourceSpan> {
        self.path.extend(path_items);
        let span = resolve_span(self.lines, self.locations, self.path.as_slice());
        self.pop_path(path_items.len());
        span.map(SourceSpan::from)
    }

    fn pop_path(&mut self, n: usize) {
        debug_assert!(self.path.len() >= n);
        self.path.truncate(self.path.len() - n);
    }

    fn bump_path(&mut self) {
        debug_assert!(self.path.len() >= 2);
        *self.path.last_mut().unwrap() += 1;
    }

    fn replace_path(&mut self, path_items: &[i32]) {
        self.pop_path(path_items.len());
        self.path.extend(path_items);
    }
}
