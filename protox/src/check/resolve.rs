use std::{
    borrow::Cow,
    collections::{hash_map, BTreeMap, HashMap},
    convert::{TryFrom, TryInto},
    fmt::{self, Write},
    mem,
    ops::RangeInclusive,
};

use miette::{Diagnostic, LabeledSpan, SourceSpan};
use prost_types::field_descriptor_proto;

use super::{names::DefinitionKind, CheckError, LineResolver, NameMap};
use crate::{
    index_to_i32,
    inversion_list::InversionList,
    make_name,
    options::{self, OptionSet},
    parse::resolve_span,
    parse_name, parse_namespace, strip_leading_dot, tag,
    types::{
        descriptor_proto, source_code_info::Location, uninterpreted_option, DescriptorProto,
        EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
        MethodDescriptorProto, OneofDescriptorProto, ServiceDescriptorProto, UninterpretedOption,
    },
    Syntax,
};

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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DuplicateNumberError {
    pub first: NumberKind,
    pub first_span: Option<SourceSpan>,
    pub second: NumberKind,
    pub second_span: Option<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NumberKind {
    EnumValue { name: String, number: i32 },
    Field { name: String, number: i32 },
    ReservedRange { start: i32, end: i32 },
    ExtensionRange { start: i32, end: i32 },
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

        self.replace_path(&[tag::message::NESTED_TYPE, 0]);
        for message in &mut message.nested_type {
            self.resolve_descriptor_proto(message);
            self.bump_path();
        }

        self.replace_path(&[tag::message::ENUM_TYPE, 0]);
        for enum_ in &mut message.enum_type {
            self.resolve_enum_descriptor_proto(enum_);
            self.bump_path();
        }

        self.replace_path(&[tag::message::EXTENSION, 0]);
        for extend in &mut message.extension {
            self.resolve_field_descriptor_proto(extend);
            self.bump_path();
        }

        self.replace_path(&[tag::message::EXTENSION_RANGE, 0]);
        for range in &mut message.extension_range {
            self.resolve_extension_range(range);
            self.bump_path();
        }

        self.replace_path(&[tag::message::ONEOF_DECL, 0]);
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

        self.check_message_field_numbers(message);
    }

    fn resolve_field_descriptor_proto(&mut self, field: &mut FieldDescriptorProto) {
        if let Some(def) = self.resolve_type_name(&mut field.extendee, &[tag::field::EXTENDEE]) {
            match def {
                DefinitionKind::Message { extension_numbers } => {
                    if !extension_numbers.contains(field.number()) {
                        let help = fmt_valid_extension_numbers_help(extension_numbers);
                        let span = self.resolve_span(&[tag::field::NUMBER]);
                        self.errors.push(CheckError::InvalidExtensionNumber {
                            number: field.number(),
                            message_name: strip_leading_dot(field.extendee()).to_owned(),
                            help,
                            span,
                        });
                    }
                }
                _ => {
                    let span = self.resolve_span(&[tag::field::EXTENDEE]);
                    self.errors.push(CheckError::InvalidExtendeeTypeName {
                        name: field.extendee().to_owned(),
                        span,
                    });
                }
            }
        }

        if let Some(def) = self.resolve_type_name(&mut field.type_name, &[tag::field::TYPE_NAME]) {
            match def {
                DefinitionKind::Message { .. } => {
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
                    if !is_valid_ident(field.default_value()) {
                        let span = self.resolve_span(&[tag::field::DEFAULT_VALUE]);
                        self.errors.push(CheckError::ValueInvalidType {
                            expected: "an enum value identifier".to_owned(),
                            actual: field.default_value().to_owned(),
                            span,
                        });
                    } else {
                        let enum_name = strip_leading_dot(field.type_name());
                        let enum_namespace = parse_namespace(enum_name);
                        let value_name = make_name(enum_namespace, field.default_value());
                        match self.name_map.get(&value_name) {
                            Some(DefinitionKind::EnumValue { parent, .. })
                                if parent == enum_name => {}
                            _ => {
                                let span = self.resolve_span(&[tag::field::DEFAULT_VALUE]);
                                self.errors.push(CheckError::InvalidEnumValue {
                                    value_name: field.default_value().to_owned(),
                                    enum_name: enum_name.to_owned(),
                                    span,
                                    help: fmt_valid_enum_values_help(self.name_map, enum_name),
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

        let allow_alias = matches!(&enum_.options, Some(o) if o.get(tag::enum_::options::ALLOW_ALIAS) == Some(&options::Value::Bool(true)));

        self.check_enum_value_numbers(enum_, allow_alias);
    }

    fn resolve_enum_value_descriptor_proto(&mut self, value: &mut EnumValueDescriptorProto) {
        self.path.push(tag::enum_value::OPTIONS);
        self.resolve_options(&mut value.options, "google.protobuf.EnumValueOptions");
        self.path.pop();
    }

    fn check_message_field_numbers(&mut self, message: &DescriptorProto) {
        #[derive(Clone)]
        enum Item {
            Field(usize),
            Range(usize),
            Extension(usize),
        }

        fn get_diagnostics(
            ctx: &mut Context,
            message: &DescriptorProto,
            item: Item,
        ) -> (NumberKind, Option<SourceSpan>) {
            match item {
                Item::Field(index) => (
                    NumberKind::Field {
                        name: message.field[index].name().to_owned(),
                        number: message.field[index].number(),
                    },
                    ctx.resolve_span(&[
                        tag::message::FIELD,
                        index_to_i32(index),
                        tag::field::NUMBER,
                    ]),
                ),
                Item::Range(index) => (
                    NumberKind::ReservedRange {
                        start: message.reserved_range[index].start(),
                        end: message.reserved_range[index].end() - 1,
                    },
                    ctx.resolve_span(&[tag::message::RESERVED_RANGE, index_to_i32(index)]),
                ),
                Item::Extension(index) => (
                    NumberKind::ExtensionRange {
                        start: message.extension_range[index].start(),
                        end: message.extension_range[index].end() - 1,
                    },
                    ctx.resolve_span(&[tag::message::EXTENSION_RANGE, index_to_i32(index)]),
                ),
            }
        }

        fn get_error(
            ctx: &mut Context,
            message: &DescriptorProto,
            (first, second): (Item, Item),
        ) -> DuplicateNumberError {
            let (first, first_span) = get_diagnostics(ctx, message, first);
            let (second, second_span) = get_diagnostics(ctx, message, second);

            DuplicateNumberError {
                first,
                first_span,
                second,
                second_span,
            }
        }

        let mut items = BTreeMap::new();

        for (index, range) in message.reserved_range.iter().enumerate() {
            if range.start() >= range.end() {
                let span = self.resolve_span(&[tag::message::RESERVED_RANGE, index_to_i32(index)]);
                self.errors.push(CheckError::InvalidRange { span });
            } else if let Err(err) = number_map_insert_range(
                &mut items,
                range.start()..=(range.end() - 1),
                Item::Range(index),
            ) {
                let error = get_error(self, message, err);
                self.errors.push(CheckError::DuplicateNumber(error));
            }
        }

        for (index, range) in message.extension_range.iter().enumerate() {
            if range.start() >= range.end() {
                let span = self.resolve_span(&[tag::message::EXTENSION_RANGE, index_to_i32(index)]);
                self.errors.push(CheckError::InvalidRange { span });
            } else if let Err(err) = number_map_insert_range(
                &mut items,
                range.start()..=(range.end() - 1),
                Item::Extension(index),
            ) {
                let error = get_error(self, message, err);
                self.errors.push(CheckError::DuplicateNumber(error));
            }
        }

        for (index, field) in message.field.iter().enumerate() {
            if let Err(err) = number_map_insert(&mut items, field.number(), Item::Field(index)) {
                let error = get_error(self, message, err);
                self.errors.push(CheckError::DuplicateNumber(error));
            }
        }
    }

    fn check_enum_value_numbers(&mut self, enum_: &EnumDescriptorProto, allow_alias: bool) {
        #[derive(Clone)]
        enum Item {
            Value(usize),
            Range(i32, usize),
        }

        fn get_diagnostics(
            ctx: &mut Context,
            enum_: &EnumDescriptorProto,
            item: Item,
        ) -> (NumberKind, Option<SourceSpan>) {
            match item {
                Item::Value(index) => (
                    NumberKind::EnumValue {
                        name: enum_.value[index].name().to_owned(),
                        number: enum_.value[index].number(),
                    },
                    ctx.resolve_span(&[
                        tag::enum_::VALUE,
                        index_to_i32(index),
                        tag::enum_value::NUMBER,
                    ]),
                ),
                Item::Range(_, index) => (
                    NumberKind::ReservedRange {
                        start: enum_.reserved_range[index].start(),
                        end: enum_.reserved_range[index].end(),
                    },
                    ctx.resolve_span(&[tag::enum_::RESERVED_RANGE, index_to_i32(index)]),
                ),
            }
        }

        fn get_error(
            ctx: &mut Context,
            enum_: &EnumDescriptorProto,
            (first, second): (Item, Item),
        ) -> DuplicateNumberError {
            let (first, first_span) = get_diagnostics(ctx, enum_, first);
            let (second, second_span) = get_diagnostics(ctx, enum_, second);

            DuplicateNumberError {
                first,
                first_span,
                second,
                second_span,
            }
        }

        let mut items = BTreeMap::new();

        for (index, range) in enum_.reserved_range.iter().enumerate() {
            if range.start() > range.end() {
                let span = self.resolve_span(&[tag::enum_::RESERVED_RANGE, index_to_i32(index)]);
                self.errors.push(CheckError::InvalidRange { span });
            } else if let Err(err) = number_map_insert_range(
                &mut items,
                range.start()..=range.end(),
                Item::Range(range.end(), index),
            ) {
                let error = get_error(self, enum_, err);
                self.errors.push(CheckError::DuplicateNumber(error));
            }
        }

        for (index, field) in enum_.value.iter().enumerate() {
            if let Err(err) = number_map_insert(&mut items, field.number(), Item::Value(index)) {
                if !allow_alias || !matches!(err, (Item::Value(_), Item::Value(_))) {
                    let error = get_error(self, enum_, err);
                    self.errors.push(CheckError::DuplicateNumber(error));
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

        self.path.push(tag::service::OPTIONS);
        self.resolve_options(&mut service.options, "google.protobuf.ServiceOptions");
        self.path.pop();

        self.pop_path(2);
        self.exit_scope();
    }

    fn resolve_method_descriptor_proto(&mut self, method: &mut MethodDescriptorProto) {
        let input_ty = self.resolve_type_name(&mut method.input_type, &[tag::method::INPUT_TYPE]);
        if !matches!(input_ty, None | Some(DefinitionKind::Message { .. })) {
            let span = self.resolve_span(&[tag::method::INPUT_TYPE]);
            self.errors.push(CheckError::InvalidMethodTypeName {
                name: method.input_type().to_owned(),
                kind: "input",
                span,
            })
        }

        let output_ty =
            self.resolve_type_name(&mut method.output_type, &[tag::method::OUTPUT_TYPE]);
        if !matches!(output_ty, None | Some(DefinitionKind::Message { .. })) {
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
            #[cfg(feature = "parse")]
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
                if let Some((
                    extension_name,
                    DefinitionKind::Field {
                        number: field_number,
                        label,
                        ty: field_ty,
                        type_name: field_type_name,
                        extendee: Some(extendee),
                        ..
                    },
                )) = self.resolve_option_def(self.scope_name(), &part.name_part)
                {
                    if extendee == namespace {
                        numbers.push(*field_number);
                        ty = *field_ty;
                        is_repeated = matches!(label, Some(Label::Repeated));
                        type_name_context =
                            Some(strip_leading_dot(parse_namespace(&extension_name)).to_owned());
                        type_name = field_type_name.clone().map(Cow::Owned);
                    } else {
                        self.errors
                            .push(CheckError::OptionExtensionInvalidExtendee {
                                extension_name: extension_name.into_owned(),
                                expected_extendee: namespace.to_owned(),
                                actual_extendee: extendee.clone(),
                                span: option_span,
                            });
                        return;
                    }
                } else {
                    self.errors.push(CheckError::OptionUnknownField {
                        name: part.name_part.clone(),
                        namespace: namespace.to_owned(),
                        span: option_span,
                    });
                    return;
                }
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
            None | Some(Type::Message | Type::Group | Type::Enum) => {
                let type_name = type_name.unwrap_or_default();
                match self.resolve_option_def(type_name_context, type_name) {
                    Some((full_type_name, DefinitionKind::Message { .. })) => {
                        let value = self.check_option_value_message(
                            option,
                            option_span,
                            strip_leading_dot(&full_type_name),
                        )?;
                        if ty == Some(Type::Group) {
                            options::Value::Group(value)
                        } else {
                            options::Value::Message(value)
                        }
                    }
                    Some((full_type_name, DefinitionKind::Enum)) => {
                        let value = self.check_option_value_enum(
                            option,
                            option_span,
                            strip_leading_dot(&full_type_name),
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
        Ok(OptionSet::default())
    }

    fn check_option_value_enum(
        &mut self,
        value: UninterpretedOption,
        span: Option<SourceSpan>,
        enum_name: &str,
    ) -> Result<i32, ()> {
        let enum_namespace = parse_namespace(enum_name);

        match value.identifier_value {
            Some(ident) => match self.get_option_def(&make_name(enum_namespace, &ident)) {
                Some(DefinitionKind::EnumValue { parent, number }) if parent == enum_name => {
                    Ok(*number)
                }
                _ => {
                    let source_name_map = if self.name_map.get(enum_name).is_some() {
                        self.name_map
                    } else {
                        NameMap::google_descriptor()
                    };

                    self.errors.push(CheckError::InvalidEnumValue {
                        value_name: ident,
                        enum_name: enum_name.to_owned(),
                        help: fmt_valid_enum_values_help(source_name_map, enum_name),
                        span,
                    });
                    Err(())
                }
            },
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

    fn resolve_option_def<'b>(
        &self,
        context: &str,
        name: &'b str,
    ) -> Option<(Cow<'b, str>, &DefinitionKind)> {
        if let Some((name, def)) = self.name_map.resolve(context, name) {
            return Some((name, def));
        }

        if !self.is_google_descriptor {
            if let Some((name, def)) = NameMap::google_descriptor().resolve(context, name) {
                return Some((name, def));
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

fn number_map_insert<T: Clone>(
    map: &mut BTreeMap<i32, (i32, T)>,
    number: i32,
    value: T,
) -> Result<(), (T, T)> {
    number_map_insert_range(map, number..=number, value)
}

fn number_map_insert_range<T: Clone>(
    map: &mut BTreeMap<i32, (i32, T)>,
    range: RangeInclusive<i32>,
    value: T,
) -> Result<(), (T, T)> {
    match map.range(..=*range.end()).last() {
        Some((&existing_start, &(existing_end, ref existing_value)))
            if range_intersects(existing_start..=existing_end, range.clone()) =>
        {
            Err((existing_value.clone(), value))
        }
        _ => {
            map.insert(*range.start(), (*range.end(), value));
            Ok(())
        }
    }
}

fn range_intersects(l: RangeInclusive<i32>, r: RangeInclusive<i32>) -> bool {
    l.start() <= r.end() && r.start() <= l.end()
}

fn is_valid_ident(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_alphabetic()
        && s.as_bytes()[1..]
            .iter()
            .all(|&ch| ch.is_ascii_alphanumeric() || ch == b'_')
}

fn to_lower_without_underscores(name: &str) -> String {
    name.chars()
        .filter_map(|ch| match ch {
            '_' => None,
            _ => Some(ch.to_ascii_lowercase()),
        })
        .collect()
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
        crate::fmt::HexEscaped(string_value.as_slice()).to_string()
    } else if let Some(aggregate_value) = &value.aggregate_value {
        format!("{{{}}}", aggregate_value)
    } else {
        String::new()
    }
}

fn fmt_valid_enum_values_help(name_map: &NameMap, enum_name: &str) -> Option<String> {
    let mut names = Vec::new();
    for (name, def) in name_map.iter() {
        if matches!(def, DefinitionKind::EnumValue { parent, .. } if parent == enum_name) {
            names.push(parse_name(name));
        }
    }

    names.sort_unstable();
    if names.is_empty() {
        None
    } else {
        let mut result = "possible values are ".to_string();
        fmt_list(&mut result, &names, |s, name| write!(s, "'{}'", name));
        Some(result)
    }
}

fn fmt_valid_extension_numbers_help(ranges: &InversionList) -> Option<String> {
    if ranges.is_empty() {
        return None;
    }

    let mut result = "available extension numbers are ".to_owned();
    fmt_list(&mut result, ranges.as_slice(), |s, range| {
        write!(s, "{} to {}", range.start, range.end - 1)
    });
    Some(result)
}

fn fmt_list<T>(result: &mut String, items: &[T], f: impl Fn(&mut String, &T) -> fmt::Result) {
    match items.len() {
        0 => (),
        1 => f(result, &items[0]).unwrap(),
        _ => {
            for value in &items[..items.len() - 2] {
                f(result, value).unwrap();
                result.push_str("', ");
            }
            f(result, &items[items.len() - 2]).unwrap();
            result.push_str(" and ");
            f(result, &items[items.len() - 1]).unwrap();
        }
    }
}

impl fmt::Display for DuplicateNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use NumberKind::*;

        match (&self.first, &self.second) {
            (
                EnumValue {
                    name: first,
                    number,
                },
                EnumValue { name: second, .. },
            ) => {
                write!(
                    f,
                    "enum number '{}' is used by both '{}' and '{}'",
                    number, first, second
                )
            }
            (
                Field {
                    name: first,
                    number,
                },
                Field { name: second, .. },
            ) => {
                write!(
                    f,
                    "field number '{}' is used by both '{}' and '{}'",
                    number, first, second
                )
            }
            (EnumValue { number, .. }, ReservedRange { .. })
            | (ReservedRange { .. }, EnumValue { number, .. }) => {
                write!(f, "enum number '{}' is marked as reserved", number)
            }
            (Field { number, .. }, ReservedRange { .. })
            | (ReservedRange { .. }, Field { number, .. }) => {
                write!(f, "field number '{}' is marked as reserved", number)
            }
            (Field { number, .. }, ExtensionRange { .. })
            | (ExtensionRange { .. }, Field { number, .. }) => {
                write!(f, "field number '{}' is marked as an extension", number)
            }
            (first, second) => write!(f, "{} overlaps with {}", first, second),
        }
    }
}

impl std::error::Error for DuplicateNumberError {}

impl Diagnostic for DuplicateNumberError {
    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        use NumberKind::*;

        match (&self.first, self.first_span, &self.second, self.second_span) {
            (
                EnumValue { .. } | Field { .. },
                Some(first_span),
                EnumValue { .. } | Field { .. },
                Some(second_span),
            ) => Some(Box::new(
                [
                    LabeledSpan::new_with_span(
                        Some("number first used here…".to_owned()),
                        first_span,
                    ),
                    LabeledSpan::new_with_span(
                        Some("…and used again here".to_owned()),
                        second_span,
                    ),
                ]
                .into_iter(),
            )),
            (
                Field { .. } | EnumValue { .. },
                Some(used_span),
                ReservedRange { .. },
                Some(reserved_span),
            )
            | (
                ReservedRange { .. },
                Some(reserved_span),
                Field { .. } | EnumValue { .. },
                Some(used_span),
            ) => Some(Box::new(
                [
                    LabeledSpan::new_with_span(Some("number used here…".to_owned()), used_span),
                    LabeledSpan::new_with_span(
                        Some("…but is marked as reserved here".to_owned()),
                        reserved_span,
                    ),
                ]
                .into_iter(),
            )),
            (Field { .. }, Some(used_span), ExtensionRange { .. }, Some(extension_span))
            | (ExtensionRange { .. }, Some(extension_span), Field { .. }, Some(used_span)) => {
                Some(Box::new(
                    [
                        LabeledSpan::new_with_span(Some("number used here…".to_owned()), used_span),
                        LabeledSpan::new_with_span(
                            Some("…but is marked an extension here".to_owned()),
                            extension_span,
                        ),
                    ]
                    .into_iter(),
                ))
            }
            (
                ReservedRange { .. } | ExtensionRange { .. },
                Some(first_span),
                ReservedRange { .. } | ExtensionRange { .. },
                Some(second_span),
            ) => Some(Box::new(
                [
                    LabeledSpan::new_with_span(Some("range defined here…".to_owned()), first_span),
                    LabeledSpan::new_with_span(
                        Some("…overlaps with range defined here".to_owned()),
                        second_span,
                    ),
                ]
                .into_iter(),
            )),
            _ => None,
        }
    }

    fn help(&self) -> Option<Box<dyn fmt::Display + '_>> {
        use NumberKind::*;

        match (&self.first, &self.second) {
            (EnumValue { .. }, EnumValue { .. }) => Some(Box::new(
                "if this is intentional, set the 'allow_alias' enum option to true",
            )),
            _ => None,
        }
    }
}

impl fmt::Display for NumberKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumberKind::EnumValue { name, .. } => write!(f, "enum value '{}'", name),
            NumberKind::Field { name, .. } => write!(f, "field '{}'", name),
            NumberKind::ReservedRange { start, end } => {
                write!(f, "reserved range '{} to {}'", start, end)
            }
            NumberKind::ExtensionRange { start, end } => {
                write!(f, "extension range '{} to {}'", start, end)
            }
        }
    }
}
