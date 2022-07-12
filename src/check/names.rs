use std::{
    borrow::Cow,
    collections::{hash_map, HashMap},
};

use logos::Span;

use crate::{
    ast,
    compile::{ParsedFile, ParsedFileMap},
};

use super::{ir, CheckError};

/// A simple map of all definitions in a proto file for checking downstream files.
#[derive(Debug)]
pub(crate) struct NameMap {
    map: HashMap<String, Entry>,
}

#[derive(Debug, Clone)]
struct Entry {
    kind: DefinitionKind,
    span: Span,
    public: bool,
    file: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DefinitionKind {
    Package,
    Message,
    Group,
    Enum,
    EnumValue {
        number: i32,
    },
    Oneof,
    Field {
        number: i32,
        ty: ast::Ty,
        label: Option<ast::FieldLabel>,
    },
    Extension {
        number: i32,
        ty: ast::Ty,
        label: Option<ast::FieldLabel>,
    },
    Service,
    Method,
}

impl NameMap {
    fn new() -> Self {
        NameMap {
            map: HashMap::new(),
        }
    }

    fn add(
        &mut self,
        name: String,
        kind: DefinitionKind,
        span: Span,
        file: Option<&str>,
        public: bool,
    ) -> Result<(), CheckError> {
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
                _ => Err({
                    let name = entry.key().clone();
                    if let Some(first_file) = &entry.get().file {
                        if let Some(second_file) = file {
                            CheckError::DuplicateNameInImports {
                                name,
                                first_file: first_file.clone(),
                                second_file: second_file.to_owned(),
                            }
                        } else {
                            CheckError::DuplicateNameInFileAndImport {
                                name,
                                first_file: first_file.clone(),
                                second: span,
                            }
                        }
                    } else {
                        CheckError::DuplicateNameInFile {
                            name,
                            first: entry.get().span.clone(),
                            second: span,
                        }
                    }
                }),
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

impl<'a> ir::File<'a> {
    pub fn get_names(&self, file_map: &ParsedFileMap) -> Result<NameMap, Vec<CheckError>> {
        let mut ctx = NamePass {
            name_map: NameMap::new(),
            errors: Vec::new(),
            scope: Vec::new(),
        };

        ctx.add_file(self, file_map);
        debug_assert!(ctx.scope.is_empty());

        if ctx.errors.is_empty() {
            Ok(ctx.name_map)
        } else {
            Err(ctx.errors)
        }
    }
}

struct NamePass {
    name_map: NameMap,
    scope: Vec<String>,
    errors: Vec<CheckError>,
}

impl NamePass {
    fn add_name<'a>(&mut self, name: impl Into<Cow<'a, str>>, kind: DefinitionKind, span: Span) {
        if let Err(err) = self
            .name_map
            .add(self.full_name(name), kind, span, None, true)
        {
            self.errors.push(err);
        }
    }

    fn merge_names(&mut self, file: &ParsedFile, public: bool) {
        if let Err(err) = self
            .name_map
            .merge(&file.name_map, file.name.clone(), public)
        {
            self.errors.push(err);
        }
    }

    fn full_name<'a>(&self, name: impl Into<Cow<'a, str>>) -> String {
        let name = name.into();
        match self.scope.last() {
            Some(namespace) => format!("{}.{}", namespace, name.as_ref()),
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
            let name = package.name.to_string();
            self.add_name(&name, DefinitionKind::Package, package.name.span());
            self.enter(&name);
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

        if file.ast.package.is_some() {
            self.exit();
        }
    }

    fn add_message(&mut self, message: &ir::Message) {
        let def = match message.ast {
            ir::MessageSource::Group(..) => DefinitionKind::Group,
            ir::MessageSource::Map(_) | ir::MessageSource::Message(_) => DefinitionKind::Message,
        };

        let name = message.ast.name();
        self.add_name(name.as_ref(), def, message.ast.name_span());
        self.enter(name);

        for field in &message.fields {
            self.add_name(
                field.ast.name(),
                DefinitionKind::Field {
                    ty: field.ast.ty(),
                    number: field.ast.number().as_i32().unwrap_or(0),
                    label: field.ast.label(),
                },
                field.ast.name_span(),
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

            self.add_name(name, DefinitionKind::Oneof, span);
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
            self.add_name(
                field.field_name(),
                DefinitionKind::Extension {
                    ty: field.ty(),
                    number: field.number.as_i32().unwrap_or(0),
                    label: field.label.clone().map(|(l, _)| l),
                },
                field.name.span.clone(),
            );
        }
    }

    fn add_enum(&mut self, enu: &ast::Enum) {
        self.add_name(&enu.name.value, DefinitionKind::Enum, enu.name.span.clone());

        for value in &enu.values {
            self.add_name(
                &value.name.value,
                DefinitionKind::EnumValue {
                    number: value.number.as_i32().unwrap_or(0),
                },
                value.name.span.clone(),
            )
        }
    }

    fn add_service(&mut self, service: &ast::Service) {
        self.add_name(
            &service.name.value,
            DefinitionKind::Service,
            service.name.span.clone(),
        );

        self.enter(&service.name.value);
        for method in &service.methods {
            self.add_name(
                &method.name.value,
                DefinitionKind::Method,
                method.name.span.clone(),
            );
        }
        self.exit();
    }
}
