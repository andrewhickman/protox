use std::{
    borrow::Cow,
    collections::{hash_map, HashMap},
    convert::{TryFrom, TryInto},
    mem,
};

use miette::SourceSpan;
use prost_types::field_descriptor_proto;

use crate::{
    ast::{HexEscaped, Syntax},
    case::to_lower_without_underscores,
    index_to_i32,
    lines::LineResolver,
    make_name,
    options::{self, OptionSet},
    parse, parse_namespace, resolve_span, strip_leading_dot, tag,
    types::{
        descriptor_proto, source_code_info::Location, uninterpreted_option, DescriptorProto,
        EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
        MethodDescriptorProto, OneofDescriptorProto, ServiceDescriptorProto, UninterpretedOption,
    },
};

use super::{names::DefinitionKind, CheckError, NameMap};

/// Resolve and check relative type names and options.
pub(crate) fn resolve(
    file: &mut FileDescriptorProto,
    lines: Option<&LineResolver>,
    name_map: &NameMap,
) -> Result<(), Vec<CheckError>> {
    let mut source_code_info = file.source_code_info.take();

    let mut tmp;
    let locations = match &mut source_code_info {
        Some(s) => &mut s.location,
        None => {
            tmp = Vec::new();
            &mut tmp
        }
    };

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
    locations: &'a mut Vec<Location>,
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

        self.replace_path(&[tag::file::ENUM_TYPE, 0]);
        for enum_ in &mut file.enum_type {
            self.resolve_enum_descriptor_proto(enum_);
            self.bump_path();
        }

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

        self.path.push(tag::file::OPTIONS);
        self.resolve_options(&mut file.options, "google.protobuf.FileOptions");
        self.path.pop();

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

        self.path.extend(&[tag::message::EXTENSION_RANGE, 0]);
        for range in &mut message.extension_range {
            self.resolve_extension_range(range);
            self.bump_path();
        }
        self.pop_path(2);

        self.path.extend(&[tag::message::ONEOF_DECL, 0]);
        for oneof in &mut message.oneof_decl {
            self.resolve_oneof_descriptor_proto(oneof);
            self.bump_path();
        }
        self.pop_path(2);

        self.path.push(tag::message::OPTIONS);
        self.resolve_options(&mut message.options, "google.protobuf.MessageOptions");
        self.path.pop();
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

        if !field.default_value().is_empty() {
            match field.r#type() {
                field_descriptor_proto::Type::Message | field_descriptor_proto::Type::Group => {
                    let span = self.resolve_span(&[tag::field::DEFAULT_VALUE]);
                    self.errors.push(CheckError::InvalidDefault {
                        kind: "message",
                        span,
                    });
                }
                field_descriptor_proto::Type::Enum => {
                    if !parse::is_valid_ident(field.default_value()) {
                        let span = self.resolve_span(&[tag::field::DEFAULT_VALUE]);
                        self.errors.push(CheckError::ValueInvalidType {
                            expected: "an enum value identifier".to_owned(),
                            actual: field.default_value().to_owned(),
                            span,
                        });
                    } else {
                        let enum_name = strip_leading_dot(field.type_name());
                        let value_name =
                            make_name(parse_namespace(enum_name), field.default_value());
                        match self.name_map.get(&value_name) {
                            Some(DefinitionKind::EnumValue { parent, .. })
                                if parent == enum_name => {}
                            _ => {
                                let span = self.resolve_span(&[tag::field::DEFAULT_VALUE]);
                                self.errors.push(CheckError::InvalidEnumValue {
                                    value_name: field.default_value().to_owned(),
                                    enum_name: enum_name.to_owned(),
                                    span,
                                });
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        self.path.push(tag::field::OPTIONS);
        self.resolve_options(&mut field.options, "google.protobuf.FieldOptions");
        self.path.pop();
    }

    fn resolve_extension_range(&mut self, range: &mut descriptor_proto::ExtensionRange) {
        self.path.push(tag::message::extension_range::OPTIONS);
        self.resolve_options(&mut range.options, "google.protobuf.ExtensionRangeOptions");
        self.path.pop();
    }

    fn resolve_oneof_descriptor_proto(&mut self, oneof: &mut OneofDescriptorProto) {
        self.path.push(tag::oneof::OPTIONS);
        self.resolve_options(&mut oneof.options, "google.protobuf.OneofOptions");
        self.path.pop();
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

    fn resolve_enum_descriptor_proto(&mut self, enum_: &mut EnumDescriptorProto) {
        self.path.extend(&[tag::enum_::VALUE, 0]);
        for value in &mut enum_.value {
            self.resolve_enum_value_descriptor_proto(value)
        }
        self.pop_path(2);

        self.path.push(tag::enum_::OPTIONS);
        self.resolve_options(&mut enum_.options, "google.protobuf.EnumOptions");
        self.path.pop();
    }

    fn resolve_enum_value_descriptor_proto(&mut self, value: &mut EnumValueDescriptorProto) {
        self.path.push(tag::enum_value::OPTIONS);
        self.resolve_options(&mut value.options, "google.protobuf.EnumValueOptions");
        self.path.pop();
    }

    fn resolve_service_descriptor_proto(&mut self, service: &mut ServiceDescriptorProto) {
        self.enter_scope(service.name());
        self.path.extend(&[tag::service::METHOD, 0]);

        for method in &mut service.method {
            self.resolve_method_descriptor_proto(method);
            self.bump_path();
        }

        self.path.push(tag::service::OPTIONS);
        self.resolve_options(&mut service.options, "google.protobuf.ServiceOptions");
        self.path.pop();

        self.pop_path(2);
        self.exit_scope();
    }

    fn resolve_method_descriptor_proto(&mut self, method: &mut MethodDescriptorProto) {
        let input_ty = self.resolve_type_name(&mut method.input_type, &[tag::method::INPUT_TYPE]);
        if !matches!(input_ty, None | Some(DefinitionKind::Message)) {
            let span = self.resolve_span(&[tag::method::INPUT_TYPE]);
            self.errors.push(CheckError::InvalidMethodTypeName {
                name: method.input_type().to_owned(),
                kind: "input",
                span,
            })
        }

        let output_ty =
            self.resolve_type_name(&mut method.output_type, &[tag::method::OUTPUT_TYPE]);
        if !matches!(output_ty, None | Some(DefinitionKind::Message)) {
            let span = self.resolve_span(&[tag::method::OUTPUT_TYPE]);
            self.errors.push(CheckError::InvalidMethodTypeName {
                name: method.output_type().to_owned(),
                kind: "output",
                span,
            })
        }

        self.path.push(tag::method::OPTIONS);
        self.resolve_options(&mut method.options, "google.protobuf.MethodOptions");
        self.path.pop();
    }

    fn resolve_options(&mut self, options: &mut Option<OptionSet>, namespace: &str) {
        let options = match options {
            Some(options) => options,
            None => return,
        };

        for (index, option) in options.take_uninterpreted().into_iter().enumerate() {
            let location = self.remove_location(&[tag::UNINTERPRETED_OPTION, index_to_i32(index)]);
            self.resolve_option(options, namespace, option, location);
        }
    }

    fn resolve_option(
        &mut self,
        mut result: &mut OptionSet,
        namespace: &str,
        mut option: UninterpretedOption,
        location: Option<Location>,
    ) {
        use field_descriptor_proto::{Label, Type};

        let option_span = match (self.lines, &location) {
            (Some(lines), Some(location)) => lines
                .resolve_proto_span(&location.span)
                .map(SourceSpan::from),
            _ => None,
        };

        let mut numbers = vec![];
        let mut ty = None;
        let mut is_repeated = false;
        let mut type_name = Some(Cow::Borrowed(namespace));
        let mut type_name_context = None;

        let option_name = mem::take(&mut option.name);
        for part in &option_name {
            let namespace = match (ty, &type_name) {
                (None | Some(Type::Message | Type::Group), Some(name)) => name.as_ref(),
                _ => {
                    self.errors
                        .push(CheckError::OptionScalarFieldAccess { span: option_span });
                    return;
                }
            };

            if let Some(&number) = numbers.last() {
                result = result.get_message_mut(number);
            }

            if part.is_extension {
                todo!()
            } else {
                let full_name = make_name(namespace, &part.name_part);
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
                        name: part.name_part.clone(),
                        namespace: namespace.to_owned(),
                        span: option_span,
                    });
                    return;
                }
            }
        }

        let value = match self.check_option_value(
            ty,
            option,
            option_span,
            type_name_context.as_deref().unwrap_or_default(),
            type_name.as_deref(),
        ) {
            Ok(value) => value,
            Err(_) => return,
        };

        if is_repeated {
            result.set_repeated(
                *numbers.last().expect("expected at least one field access"),
                value,
            )
        } else if let Err(()) = result.set(
            *numbers.last().expect("expected at least one field access"),
            value,
        ) {
            let first = self.resolve_span(&numbers);
            self.errors.push(CheckError::OptionAlreadySet {
                name: fmt_option_name(&option_name),
                first,
                second: option_span,
            });
            return;
        }
        self.add_location(&numbers, location);
    }

    fn check_option_value(
        &mut self,
        ty: Option<field_descriptor_proto::Type>,
        option: UninterpretedOption,
        option_span: Option<SourceSpan>,
        type_name_context: &str,
        type_name: Option<&str>,
    ) -> Result<options::Value, ()> {
        use field_descriptor_proto::Type;

        Ok(match ty {
            Some(Type::Double) => {
                options::Value::Double(self.check_option_value_f64(option, option_span)?)
            }
            Some(Type::Float) => {
                options::Value::Float(self.check_option_value_f64(option, option_span)? as f32)
            }
            Some(Type::Int32) => {
                options::Value::Int32(self.check_option_value_i32(option, option_span)?)
            }
            Some(Type::Int64) => {
                options::Value::Int64(self.check_option_value_i64(option, option_span)?)
            }
            Some(Type::Uint32) => {
                options::Value::Uint32(self.check_option_value_u32(option, option_span)?)
            }
            Some(Type::Uint64) => {
                options::Value::Uint64(self.check_option_value_u64(option, option_span)?)
            }
            Some(Type::Sint32) => {
                options::Value::Sint32(self.check_option_value_i32(option, option_span)?)
            }
            Some(Type::Sint64) => {
                options::Value::Sint64(self.check_option_value_i64(option, option_span)?)
            }
            Some(Type::Fixed32) => {
                options::Value::Fixed32(self.check_option_value_u32(option, option_span)?)
            }
            Some(Type::Fixed64) => {
                options::Value::Fixed64(self.check_option_value_u64(option, option_span)?)
            }
            Some(Type::Sfixed32) => {
                options::Value::Sfixed32(self.check_option_value_i32(option, option_span)?)
            }
            Some(Type::Sfixed64) => {
                options::Value::Sfixed64(self.check_option_value_i64(option, option_span)?)
            }
            Some(Type::Bool) => {
                options::Value::Bool(self.check_option_value_bool(option, option_span)?)
            }
            Some(Type::String) => {
                options::Value::String(self.check_option_value_string(option, option_span)?)
            }
            Some(Type::Bytes) => {
                options::Value::Bytes(self.check_option_value_bytes(option, option_span)?)
            }
            Some(Type::Enum) => {
                let value = self.check_option_value_enum(
                    option,
                    option_span,
                    type_name_context,
                    type_name.unwrap_or_default(),
                )?;
                options::Value::Int32(value)
            }
            None | Some(Type::Message | Type::Group) => {
                let type_name = type_name.unwrap_or_default();
                match self.resolve_option_def(type_name_context, type_name) {
                    Some(DefinitionKind::Message) => {
                        let value =
                            self.check_option_value_message(option, option_span, type_name)?;
                        if ty == Some(Type::Group) {
                            options::Value::Group(value)
                        } else {
                            options::Value::Message(value)
                        }
                    }
                    Some(DefinitionKind::Enum) => {
                        let value = self.check_option_value_enum(
                            option,
                            option_span,
                            type_name_context,
                            type_name,
                        )?;
                        options::Value::Int32(value)
                    }
                    Some(_) | None => {
                        self.errors.push(CheckError::OptionInvalidTypeName {
                            name: type_name.to_owned(),
                            span: option_span,
                        });
                        return Err(());
                    }
                }
            }
        })
    }

    fn check_option_value_f64(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
    ) -> Result<f64, ()> {
        if let Some(float) = value.double_value {
            Ok(float)
        } else if let Some(int) = value.positive_int_value {
            Ok(int as f64)
        } else if let Some(int) = value.negative_int_value {
            Ok(int as f64)
        } else {
            self.errors.push(CheckError::ValueInvalidType {
                expected: "a number".to_owned(),
                actual: fmt_option_value(&value),
                span,
            });
            Err(())
        }
    }

    fn check_option_value_i32(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
    ) -> Result<i32, ()> {
        match self.check_option_value_int(&value, span)? {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "a signed 32-bit integer".to_owned(),
                    actual: fmt_option_value(&value),
                    min: i32::MIN.to_string(),
                    max: i32::MAX.to_string(),
                    span,
                });
                Err(())
            }
        }
    }

    fn check_option_value_i64(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
    ) -> Result<i64, ()> {
        match self.check_option_value_int(&value, span)? {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "a signed 64-bit integer".to_owned(),
                    actual: fmt_option_value(&value),
                    min: i64::MIN.to_string(),
                    max: i64::MAX.to_string(),
                    span,
                });
                Err(())
            }
        }
    }

    fn check_option_value_u32(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
    ) -> Result<u32, ()> {
        match self.check_option_value_int(&value, span)? {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "an unsigned 32-bit integer".to_owned(),
                    actual: fmt_option_value(&value),
                    min: u32::MIN.to_string(),
                    max: u32::MAX.to_string(),
                    span,
                });
                Err(())
            }
        }
    }

    fn check_option_value_u64(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
    ) -> Result<u64, ()> {
        match self.check_option_value_int(&value, span)? {
            Some(value) => Ok(value),
            None => {
                self.errors.push(CheckError::IntegerValueOutOfRange {
                    expected: "an unsigned 64-bit integer".to_owned(),
                    actual: fmt_option_value(&value),
                    min: u64::MIN.to_string(),
                    max: u64::MAX.to_string(),
                    span,
                });
                Err(())
            }
        }
    }

    fn check_option_value_int<T>(
        &mut self,
        value: &UninterpretedOption,
        span: Option<SourceSpan>,
    ) -> Result<Option<T>, ()>
    where
        T: TryFrom<u64> + TryFrom<i64>,
    {
        if let Some(int) = value.positive_int_value {
            Ok(int.try_into().ok())
        } else if let Some(int) = value.negative_int_value {
            Ok(int.try_into().ok())
        } else {
            self.errors.push(CheckError::ValueInvalidType {
                expected: "an integer".to_owned(),
                actual: fmt_option_value(value),
                span,
            });
            Err(())
        }
    }

    fn check_option_value_bool(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
    ) -> Result<bool, ()> {
        match value.identifier_value() {
            "false" => Ok(false),
            "true" => Ok(true),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "either 'true' or 'false'".to_owned(),
                    actual: fmt_option_value(&value),
                    span,
                });
                Err(())
            }
        }
    }

    fn check_option_value_string(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
    ) -> Result<String, ()> {
        let bytes = self.check_option_value_bytes(value, span)?;
        match String::from_utf8(bytes) {
            Ok(string) => Ok(string),
            Err(_) => {
                self.errors
                    .push(CheckError::StringValueInvalidUtf8 { span });
                Err(())
            }
        }
    }

    fn check_option_value_bytes(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
    ) -> Result<Vec<u8>, ()> {
        match value.string_value {
            Some(string) => Ok(string),
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "a string".to_owned(),
                    actual: fmt_option_value(&value),
                    span,
                });
                Err(())
            }
        }
    }

    fn check_option_value_message(
        &mut self,
        _value: UninterpretedOption,
        _span: Option<SourceSpan>,
        _type_name: &str,
    ) -> Result<OptionSet, ()> {
        todo!()
    }

    fn check_option_value_enum(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
        context: &str,
        type_name: &str,
    ) -> Result<i32, ()> {
        let type_namespace = parse_namespace(type_name);

        match value.identifier_value {
            Some(ident) => {
                match self.resolve_option_def(context, &make_name(type_namespace, &ident)) {
                    Some(DefinitionKind::EnumValue { parent, number }) if parent == type_name => {
                        Ok(*number)
                    }
                    _ => {
                        self.errors.push(CheckError::InvalidEnumValue {
                            value_name: ident,
                            enum_name: type_name.to_owned(),
                            span,
                        });
                        Err(())
                    }
                }
            }
            _ => {
                self.errors.push(CheckError::ValueInvalidType {
                    expected: "an enum value identifier".to_owned(),
                    actual: fmt_option_value(&value),
                    span,
                });
                Err(())
            }
        }
    }

    fn get_option_def(&self, name: &str) -> Option<&DefinitionKind> {
        if let Some(def) = self.name_map.get(name) {
            return Some(def);
        }

        if !self.is_google_descriptor {
            return NameMap::google_descriptor().get(name);
        }

        None
    }

    fn resolve_option_def(&self, context: &str, name: &str) -> Option<&DefinitionKind> {
        if let Some((_, def)) = self.name_map.resolve(context, name) {
            return Some(def);
        }

        if !self.is_google_descriptor {
            if let Some((_, def)) = NameMap::google_descriptor().resolve(context, name) {
                return Some(def);
            }
        }

        None
    }

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

    fn add_location(&mut self, path_items: &[i32], location: Option<Location>) {
        if let Some(mut location) = location {
            self.path.extend(path_items);

            location.path = self.path.clone();
            match self
                .locations
                .binary_search_by(|l| l.path.as_slice().cmp(&self.path))
            {
                Ok(index) | Err(index) => self.locations.insert(index, location),
            };
            self.pop_path(path_items.len());
        }
    }

    fn remove_location(&mut self, path_items: &[i32]) -> Option<Location> {
        self.path.extend(path_items);
        let location = match self
            .locations
            .binary_search_by(|l| l.path.as_slice().cmp(&self.path))
        {
            Ok(index) => Some(self.locations.remove(index)),
            Err(_) => None,
        };
        self.pop_path(path_items.len());

        location
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

fn fmt_option_name(name: &[uninterpreted_option::NamePart]) -> String {
    let mut result = String::new();
    for part in name {
        if !result.is_empty() {
            result.push('.');
        }
        if part.is_extension {
            result.push('(');
            result.push_str(&part.name_part);
            result.push(')');
        } else {
            result.push_str(&part.name_part);
        }
    }
    result
}

fn fmt_option_value(value: &UninterpretedOption) -> String {
    if let Some(identifier_value) = &value.identifier_value {
        identifier_value.clone()
    } else if let Some(positive_int_value) = value.positive_int_value {
        positive_int_value.to_string()
    } else if let Some(negative_int_value) = value.negative_int_value {
        negative_int_value.to_string()
    } else if let Some(double_value) = value.double_value {
        double_value.to_string()
    } else if let Some(string_value) = &value.string_value {
        HexEscaped(string_value.as_slice()).to_string()
    } else if let Some(aggregate_value) = &value.aggregate_value {
        format!("{{{}}}", aggregate_value)
    } else {
        String::new()
    }
}
