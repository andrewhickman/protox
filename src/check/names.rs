use std::{
    borrow::Cow,
    collections::{hash_map, HashMap},
    fmt,
    iter::once,
};

use logos::Span;
use miette::{Diagnostic, LabeledSpan};

use crate::{
    ast,
    compile::{ParsedFile, ParsedFileMap},
    index_to_i32,
    types::{
        field_descriptor_proto, DescriptorProto, EnumDescriptorProto, FieldDescriptorProto,
        FileDescriptorProto, OneofDescriptorProto, ServiceDescriptorProto,
    }, make_name,
};

use super::{ir, CheckError};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DuplicateNameError {
    pub name: String,
    pub first: NameLocation,
    pub second: NameLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NameLocation {
    Import(String),
    Root(Span),
    Unknown,
}

/// A simple map of all definitions in a proto file for checking downstream files.
#[derive(Debug)]
pub(crate) struct NameMap {
    map: HashMap<String, Entry>,
}

#[derive(Debug, Clone)]
struct Entry {
    kind: DefinitionKind,
    span: Option<Span>,
    public: bool,
    file: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DefinitionKind {
    Package,
    Message,
    Enum,
    EnumValue {
        number: i32,
    },
    Oneof,
    Field {
        number: i32,
        ty: Option<field_descriptor_proto::Type>,
        type_name: Option<String>,
        label: Option<field_descriptor_proto::Label>,
        oneof_index: Option<i32>,
        extendee: Option<String>,
    },
    Service,
    Method,
}

impl NameMap {
    pub fn from_ir(ir: &ir::File, file_map: &ParsedFileMap) -> Result<NameMap, Vec<CheckError>> {
        let mut ctx = NamePass {
            name_map: NameMap::new(),
            errors: Vec::new(),
            scope: Vec::new(),
        };

        ctx.add_file(ir, file_map);
        debug_assert!(ctx.scope.is_empty());

        if ctx.errors.is_empty() {
            Ok(ctx.name_map)
        } else {
            Err(ctx.errors)
        }
    }

    pub fn from_proto(
        file: &FileDescriptorProto,
        file_map: &ParsedFileMap,
    ) -> Result<NameMap, Vec<CheckError>> {
        let mut ctx = NamePass {
            name_map: NameMap::new(),
            errors: Vec::new(),
            scope: Vec::new(),
        };

        ctx.add_file_descriptor_proto(file, file_map);
        debug_assert!(ctx.scope.is_empty());

        if ctx.errors.is_empty() {
            Ok(ctx.name_map)
        } else {
            Err(ctx.errors)
        }
    }

    fn new() -> Self {
        NameMap {
            map: HashMap::new(),
        }
    }

    fn add(
        &mut self,
        name: String,
        kind: DefinitionKind,
        span: Option<Span>,
        file: Option<&str>,
        public: bool,
    ) -> Result<(), DuplicateNameError> {
        match self.map.entry(name) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(Entry {
                    file: file.map(ToOwned::to_owned),
                    kind,
                    span,
                    public,
                });
                Ok(())
            }
            hash_map::Entry::Occupied(entry) => match (kind, &entry.get().kind) {
                (DefinitionKind::Package, DefinitionKind::Package) => Ok(()),
                _ => {
                    let first =
                        NameLocation::new(entry.get().file.clone(), entry.get().span.clone());
                    let second = NameLocation::new(file.map(ToOwned::to_owned), span);

                    Err(DuplicateNameError {
                        name: entry.key().to_owned(),
                        first,
                        second,
                    })
                }
            },
        }
    }

    fn merge(&mut self, other: &Self, file: String, public: bool) -> Result<(), CheckError> {
        for (name, entry) in &other.map {
            if entry.public {
                self.add(
                    name.clone(),
                    entry.kind.clone(),
                    entry.span.clone(),
                    Some(&file),
                    public,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn get(&self, name: &str) -> Option<&DefinitionKind> {
        self.map.get(name).map(|e| &e.kind)
    }
}

struct NamePass {
    name_map: NameMap,
    scope: Vec<String>,
    errors: Vec<CheckError>,
}

impl NamePass {
    fn add_name<'a>(
        &mut self,
        name: impl Into<Cow<'a, str>>,
        kind: DefinitionKind,
        span: Option<Span>,
    ) {
        if let Err(err) = self
            .name_map
            .add(self.full_name(name), kind, span, None, true)
        {
            self.errors.push(CheckError::DuplicateName(err));
        }
    }

    fn merge_names(&mut self, file: &ParsedFile, public: bool) {
        if let Err(err) = self
            .name_map
            .merge(&file.name_map, file.name().to_owned(), public)
        {
            self.errors.push(err);
        }
    }

    fn full_name<'a>(&self, name: impl Into<Cow<'a, str>>) -> String {
        let name = name.into();
        match self.scope.last() {
            Some(namespace) => make_name(namespace, name),
            None => name.into_owned(),
        }
    }

    fn enter<'a>(&mut self, name: impl Into<Cow<'a, str>>) {
        self.scope.push(self.full_name(name))
    }

    fn exit(&mut self) {
        self.scope.pop().expect("unbalanced scope stack");
    }

    fn add_file(&mut self, file: &ir::File, file_map: &ParsedFileMap) {
        for import in &file.ast.imports {
            let file = &file_map[import.value.as_str()];
            self.merge_names(
                file,
                matches!(import.kind, Some((ast::ImportKind::Public, _))),
            );
        }

        if let Some(package) = &file.ast.package {
            for part in &package.name.parts {
                self.add_name(
                    &part.value,
                    DefinitionKind::Package,
                    Some(package.name.span().start..part.span.end),
                );
                self.enter(&part.value);
            }
        }

        for message in &file.messages {
            self.add_message(message);
        }

        for item in &file.ast.items {
            match item {
                ast::FileItem::Message(_) => continue,
                ast::FileItem::Enum(enu) => self.add_enum(enu),
                ast::FileItem::Extend(extend) => self.add_extend(extend),
                ast::FileItem::Service(service) => self.add_service(service),
            }
        }

        if let Some(package) = &file.ast.package {
            for _ in &package.name.parts {
                self.exit();
            }
        }
    }

    fn add_message(&mut self, message: &ir::Message) {
        let name = message.ast.name();
        self.add_name(
            name.as_ref(),
            DefinitionKind::Message,
            Some(message.ast.name_span()),
        );
        self.enter(name);

        for field in &message.fields {
            let ty = field.ast.ty();
            self.add_name(
                field.ast.name(),
                DefinitionKind::Field {
                    ty: ty.proto_ty(),
                    type_name: ty.ty_name(),
                    number: field.ast.number().as_i32().unwrap_or(0),
                    label: field.ast.label().map(|l| l.proto_label()),
                    oneof_index: field.oneof_index,
                    extendee: None,
                },
                Some(field.ast.name_span()),
            );
        }

        for oneof in &message.oneofs {
            let (name, span) = match oneof.ast {
                ir::OneofSource::Oneof(oneof) => (
                    Cow::Borrowed(oneof.name.value.as_str()),
                    oneof.name.span.clone(),
                ),
                ir::OneofSource::Field(field) => (
                    Cow::Owned(field.synthetic_oneof_name()),
                    field.name.span.clone(),
                ),
            };

            self.add_name(name, DefinitionKind::Oneof, Some(span));
        }

        for nested_message in &message.messages {
            self.add_message(nested_message);
        }

        if let Some(body) = message.ast.body() {
            for item in &body.items {
                match item {
                    ast::MessageItem::Enum(enu) => {
                        self.add_enum(enu);
                    }
                    ast::MessageItem::Extend(extend) => {
                        self.add_extend(extend);
                    }
                    ast::MessageItem::Field(_)
                    | ast::MessageItem::Message(_)
                    | ast::MessageItem::Oneof(_) => continue,
                }
            }
        }

        self.exit();
    }

    fn add_extend(&mut self, extend: &ast::Extend) {
        for field in &extend.fields {
            let ty = field.ty();
            self.add_name(
                field.field_name(),
                DefinitionKind::Field {
                    ty: ty.proto_ty(),
                    type_name: ty.ty_name(),
                    number: field.number.as_i32().unwrap_or(0),
                    label: field.label.clone().map(|(l, _)| l.proto_label()),
                    oneof_index: None,
                    extendee: Some(extend.extendee.to_string()),
                },
                Some(field.name.span.clone()),
            );
        }
    }

    fn add_enum(&mut self, enu: &ast::Enum) {
        self.add_name(
            &enu.name.value,
            DefinitionKind::Enum,
            Some(enu.name.span.clone()),
        );

        for value in &enu.values {
            self.add_name(
                &value.name.value,
                DefinitionKind::EnumValue {
                    number: value.number.as_i32().unwrap_or(0),
                },
                Some(value.name.span.clone()),
            )
        }
    }

    fn add_service(&mut self, service: &ast::Service) {
        self.add_name(
            &service.name.value,
            DefinitionKind::Service,
            Some(service.name.span.clone()),
        );

        self.enter(&service.name.value);
        for method in &service.methods {
            self.add_name(
                &method.name.value,
                DefinitionKind::Method,
                Some(method.name.span.clone()),
            );
        }
        self.exit();
    }

    fn add_file_descriptor_proto(&mut self, file: &FileDescriptorProto, file_map: &ParsedFileMap) {
        for (index, import) in file.dependency.iter().enumerate() {
            let import_file = &file_map[import.as_str()];
            self.merge_names(
                import_file,
                file.public_dependency.contains(&index_to_i32(index)),
            );
        }

        for part in file.package().split('.') {
            self.add_name(part, DefinitionKind::Package, None);
            self.enter(part);
        }

        for message in &file.message_type {
            self.add_descriptor_proto(message);
        }

        for enu in &file.enum_type {
            self.add_enum_descriptor_proto(enu);
        }

        for extend in &file.extension {
            self.add_field_descriptor_proto(extend);
        }

        for service in &file.service {
            self.add_service_descriptor_proto(service);
        }

        for _ in file.package().split('.') {
            self.exit();
        }
    }

    fn add_descriptor_proto(&mut self, message: &DescriptorProto) {
        self.add_name(message.name(), DefinitionKind::Message, None);
        self.enter(message.name());

        for field in &message.field {
            self.add_field_descriptor_proto(field)
        }

        for oneof in &message.oneof_decl {
            self.add_oneof_descriptor_proto(oneof);
        }

        for message in &message.nested_type {
            self.add_descriptor_proto(message);
        }

        for enu in &message.enum_type {
            self.add_enum_descriptor_proto(enu);
        }

        for extension in &message.extension {
            self.add_field_descriptor_proto(extension);
        }

        self.exit();
    }

    fn add_field_descriptor_proto(&mut self, field: &FieldDescriptorProto) {
        self.add_name(
            field.name(),
            DefinitionKind::Field {
                ty: field_descriptor_proto::Type::from_i32(field.r#type.unwrap_or(0)),
                type_name: field.type_name.clone(),
                number: field.number(),
                label: field_descriptor_proto::Label::from_i32(field.label.unwrap_or(0)),
                oneof_index: field.oneof_index,
                extendee: field.extendee.clone(),
            },
            None,
        );
    }

    fn add_oneof_descriptor_proto(&mut self, oneof: &OneofDescriptorProto) {
        self.add_name(oneof.name(), DefinitionKind::Oneof, None);
    }

    fn add_enum_descriptor_proto(&mut self, enu: &EnumDescriptorProto) {
        self.add_name(enu.name(), DefinitionKind::Enum, None);

        for value in &enu.value {
            self.add_name(
                value.name(),
                DefinitionKind::EnumValue {
                    number: value.number(),
                },
                None,
            );
        }
    }

    fn add_service_descriptor_proto(&mut self, service: &ServiceDescriptorProto) {
        self.add_name(service.name(), DefinitionKind::Service, None);

        self.enter(service.name());
        for method in &service.method {
            self.add_name(method.name(), DefinitionKind::Method, None);
        }
        self.exit();
    }
}

impl NameLocation {
    fn new(file: Option<String>, span: Option<Span>) -> NameLocation {
        match (file, span) {
            (Some(file), _) => NameLocation::Import(file),
            (None, Some(span)) => NameLocation::Root(span),
            (None, None) => NameLocation::Unknown,
        }
    }
}

impl fmt::Display for DuplicateNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.first, &self.second) {
            (NameLocation::Import(first), NameLocation::Import(second)) => write!(
                f,
                "name '{}' is defined both in imported file '{}' and '{}'",
                self.name, first, second
            ),
            (NameLocation::Import(first), NameLocation::Root(_) | NameLocation::Unknown) => write!(
                f,
                "name '{}' is already defined in imported file '{}'",
                self.name, first
            ),
            _ => write!(f, "name '{}' is defined twice", self.name),
        }
    }
}

impl std::error::Error for DuplicateNameError {}

impl Diagnostic for DuplicateNameError {
    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        match (&self.first, &self.second) {
            (NameLocation::Root(first), NameLocation::Root(second)) => Some(Box::new(
                vec![
                    LabeledSpan::new_with_span(
                        Some("first defined here…".to_owned()),
                        first.clone(),
                    ),
                    LabeledSpan::new_with_span(Some("…and again here".to_owned()), second.clone()),
                ]
                .into_iter(),
            )),
            (_, NameLocation::Root(span)) | (NameLocation::Root(span), _) => Some(Box::new(once(
                LabeledSpan::new_with_span(Some("defined here".to_owned()), span.clone()),
            ))),
            _ => None,
        }
    }
}
