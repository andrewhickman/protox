use std::collections::{hash_map, HashMap};

use logos::Span;

use super::CheckError;

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
    pub fn new() -> Self {
        NameMap {
            map: HashMap::new(),
        }
    }

    pub(super) fn add(
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

    pub(super) fn merge(
        &mut self,
        other: &Self,
        file: String,
        public: bool,
    ) -> Result<(), CheckError> {
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

// pub(super) struct NamePass<'a, 'b> {
//     pub ctx: &'a mut Context<'b>,
//     pub camel_case_field_names: HashMap<String, (String, Span)>,
// }

// impl<'a, 'b> ast::Visitor for NamePass<'a, 'b> {
//     fn visit_file(&mut self, file: &ast::File) {
//         if let Some(file_map) = &self.ctx.file_map {
//             for import in &file.imports {
//                 let file = &file_map[import.value.value.as_str()];
//                 if let Err(err) = self.ctx.names.merge(
//                     &file.name_map,
//                     file.name.clone(),
//                     import.kind == Some(ast::ImportKind::Public),
//                 ) {
//                     self.ctx.errors.push(err);
//                 }
//             }
//         }

//         if let Some(package) = &file.package {
//             self.ctx.add_name(
//                 &package.name.to_string(),
//                 DefinitionKind::Package,
//                 package.name.span(),
//             );
//         }

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
