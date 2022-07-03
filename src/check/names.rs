use std::{
    borrow::{Borrow, Cow},
    collections::{hash_map, HashMap},
};

use logos::Span;

use crate::{
    ast,
    case::to_pascal_case,
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum DefinitionKind {
    Package,
    Message,
    Enum,
    EnumValue,
    Group,
    Oneof,
    Field,
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
            hash_map::Entry::Occupied(entry) => match (kind, entry.get().kind) {
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
                    entry.kind,
                    entry.span.clone(),
                    Some(&file),
                    public,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn get(&self, name: &str) -> Option<DefinitionKind> {
        let name = name.strip_prefix('.').unwrap_or(name);
        self.map.get(name).map(|e| e.kind)
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
        match self.scope.first() {
            Some(namespace) => format!("{}.{}", namespace, name.as_ref()),
            None => name.into_owned(),
        }
    }

    fn scope_name(&self) -> &str {
        match self.scope.first() {
            Some(name) => name.as_str(),
            None => "",
        }
    }

    fn enter<'a>(&mut self, name: impl Into<Cow<'a, str>>) {
        self.scope.push(self.full_name(name))
    }

    fn exit(&mut self) {
        self.scope.pop().unwrap();
    }

    fn add_file(&mut self, file: &ir::File, file_map: &ParsedFileMap) {
        for import in &file.ast.imports {
            let file = &file_map[import.value.value.as_str()];
            self.merge_names(file, import.kind == Some(ast::ImportKind::Public));
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
                ast::FileItem::Enum(_) => todo!(),
                ast::FileItem::Extend(_) => todo!(),
                ast::FileItem::Service(_) => todo!(),
            }
        }

        if file.ast.package.is_some() {
            self.exit();
        }
    }

    fn add_message(&mut self, message: &ir::Message) {
        let (name, span) = match message.ast {
            ir::MessageSource::Message(message) => (
                Cow::Borrowed(message.name.value.as_str()),
                message.name.span.clone(),
            ),
            ir::MessageSource::Group(group) => (
                Cow::Borrowed(group.name.value.as_str()),
                group.name.span.clone(),
            ),
            ir::MessageSource::Map(map) => (Cow::Owned(map.message_name()), map.name.span.clone()),
        };

        self.add_name(name.as_ref(), DefinitionKind::Message, span);
        self.enter(name);

        for field in &message.fields {
            let (name, span) = match field.ast {
                ir::FieldSource::Field(field) => (
                    Cow::Borrowed(field.name.value.as_str()),
                    field.name.span.clone(),
                ),
                ir::FieldSource::Group(group) => {
                    (Cow::Owned(group.field_name()), group.name.span.clone())
                }
                ir::FieldSource::Map(map) => (
                    Cow::Borrowed(map.name.value.as_str()),
                    map.name.span.clone(),
                ),
            };

            self.add_name(name, DefinitionKind::Field, span);
        }

        for oneof in &message.oneofs {
            let (name, span) = match oneof.ast {
                ir::OneofSource::Oneof(oneof) => (Cow::Borrowed(oneof.name.value.as_str()), oneof.name.span.clone()),
                ir::OneofSource::Field(field) => (Cow::Owned(field.synthetic_oneof_name()), field.name.span.clone()),
            };

            self.add_name(name, DefinitionKind::Oneof, span);
        }

        for nested_message in &message.messages {
            self.add_message(nested_message);
        }

        for item in &message.ast.

        self.exit();
    }
}

//         if let Some(package) = &file.package {
//             self.ctx.enter(Definition::Package {
//                 full_name: package.name.to_string(),
//             });
//         }

//         file.visit(self);

//         if file.package.is_some() {
//             self.ctx.exit();
//         }
//     }

//     fn visit_enum(&mut self, enu: &ast::Enum) {
//         self.ctx
//             .add_name(&enu.name.value, DefinitionKind::Enum, enu.name.span.clone());
//         self.ctx.enter(Definition::Enum);
//         enu.visit(self);
//         self.ctx.exit();
//     }

//     fn visit_enum_value(&mut self, value: &ast::EnumValue) {
//         self.ctx.add_name(
//             &value.name.value,
//             DefinitionKind::EnumValue,
//             value.name.span.clone(),
//         );
//     }

//     fn visit_message(&mut self, message: &ast::Message) {
//         self.ctx.add_name(
//             &message.name.value,
//             DefinitionKind::Message,
//             message.name.span.clone(),
//         );

//         self.ctx.enter(Definition::Message {
//             full_name: self.ctx.full_name(&message.name.value),
//         });
//         debug_assert!(self.camel_case_field_names.is_empty());

//         message.body.visit(self);

//         self.camel_case_field_names.clear();
//         self.ctx.exit();
//     }

//     fn visit_field(&mut self, field: &ast::Field) {
//         self.add_field_name(&field.name.value, field.name.span.clone());
//     }

//     fn visit_map(&mut self, map: &ast::Map) {
//         self.add_field_name(&map.name.value, map.name.span.clone());
//         self.ctx.add_name(
//             &(to_pascal_case(&map.name.value) + "Entry"),
//             DefinitionKind::Message,
//             map.name.span.clone(),
//         );
//     }

//     fn visit_group(&mut self, group: &ast::Group) {
//         self.ctx.add_name(
//             &group.name.value,
//             DefinitionKind::Group,
//             group.name.span.clone(),
//         );

//         self.ctx.enter(Definition::Group);
//         group.body.visit(self);
//         self.ctx.exit();
//     }

//     fn visit_oneof(&mut self, oneof: &ast::Oneof) {
//         self.ctx.add_name(
//             &oneof.name.value,
//             DefinitionKind::Oneof,
//             oneof.name.span.clone(),
//         );

//         self.ctx.enter(Definition::Group);
//         oneof.visit(self);
//         self.ctx.exit();
//     }

//     fn visit_service(&mut self, service: &ast::Service) {
//         self.ctx.add_name(
//             &service.name.value,
//             DefinitionKind::Service,
//             service.name.span.clone(),
//         );

//         self.ctx.enter(Definition::Service {
//             full_name: self.ctx.full_name(&service.name.value),
//         });
//         service.visit(self);
//         self.ctx.exit();
//     }

//     fn visit_method(&mut self, method: &ast::Method) {
//         self.ctx.add_name(
//             &method.name.value,
//             DefinitionKind::Method,
//             method.name.span.clone(),
//         );
//     }
// }

// impl<'a, 'b> NamePass<'a, 'b> {
//     fn add_field_name(&mut self, name: &str, span: Span) {
//         self.ctx.add_name(name, DefinitionKind::Field, span.clone());
//         if self.ctx.syntax == ast::Syntax::Proto3 {
//             match self.camel_case_field_names.entry(to_lower_camel_case(name)) {
//                 hash_map::Entry::Occupied(entry) => {
//                     self.ctx
//                         .errors
//                         .push(CheckError::DuplicateCamelCaseFieldName {
//                             first_name: entry.get().0.clone(),
//                             first: entry.get().1.clone(),
//                             second_name: name.to_owned(),
//                             second: span,
//                         })
//                 }
//                 hash_map::Entry::Vacant(entry) => {
//                     entry.insert((name.to_owned(), span));
//                 }
//             }
//         }
//     }
// }
