use prost_types::{DescriptorProto, FileDescriptorProto, EnumDescriptorProto, FieldDescriptorProto, ServiceDescriptorProto, FileOptions};

use crate::{ast, index_to_i32};

use super::{ir, CheckError, NameMap};

impl<'a> ir::File<'a> {
    pub fn check(
        &self,
        name_map: Option<&NameMap>,
    ) -> Result<FileDescriptorProto, Vec<CheckError>> {
        let mut context = Context {
            syntax: self.ast.syntax,
            name_map,
            scope: Vec::new(),
            errors: Vec::new(),
        };

        let file = context.check_file(self);

        debug_assert!(context.scope.is_empty());

        if context.errors.is_empty() {
            Ok(file)
        } else {
            Err(context.errors)
        }
    }
}

struct Context<'a> {
    syntax: ast::Syntax,
    name_map: Option<&'a NameMap>,
    scope: Vec<Scope>,
    errors: Vec<CheckError>,
}

enum Scope {
    Package { full_name: String },
    Message { full_name: String },
    Enum,
    Service { full_name: String },
    Oneof,
    Extend { extendee: String },
    Group,
}

impl<'a> Context<'a> {
    fn enter(&mut self, scope: Scope) {
        self.scope.push(scope);
    }

    fn exit(&mut self) {
        self.scope.pop().expect("unbalanced scope stack");
    }

    fn check_file(&mut self, file: &ir::File) -> FileDescriptorProto {
        if let Some(package) = &file.ast.package {
            self.enter(Scope::Package {
                full_name: package.name.to_string(),
            });
        }

        let package = file.ast.package.as_ref().map(|p| p.name.to_string());

        let dependency = file
            .ast
            .imports
            .iter()
            .map(|i| i.value.value.clone())
            .collect();
        let public_dependency = file
            .ast
            .imports
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == Some(ast::ImportKind::Public))
            .map(|(index, _)| index_to_i32(index))
            .collect();
        let weak_dependency = file
            .ast
            .imports
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == Some(ast::ImportKind::Weak))
            .map(|(index, _)| index_to_i32(index))
            .collect();

        let mut message_type = file
            .messages
            .iter()
            .map(|message| self.check_message(message))
            .collect();

        let mut enum_type = Vec::new();
        let mut service = Vec::new();
        let mut extension = Vec::new();

        for item in &file.ast.items {
            match item {
                ast::FileItem::Message(_) => continue,
                ast::FileItem::Enum(e) => enum_type.push(self.check_enum(e)),
                ast::FileItem::Extend(e) => extension.push(self.check_extend(e)),
                ast::FileItem::Service(s) => service.push(self.check_service(s)),
            }
        }

        let options = self.check_file_options(&file.ast.options);

        let syntax = if self.syntax == ast::Syntax::default() {
            None
        } else {
            Some(self.syntax.to_string())
        };

        if file.ast.package.is_some() {
            self.exit();
        }

        FileDescriptorProto {
            name: None,
            package,
            dependency,
            public_dependency,
            weak_dependency,
            message_type,
            enum_type,
            service,
            extension,
            options,
            source_code_info: None,
            syntax,
        }
    }

    fn check_message(&self, message: &ir::Message) -> DescriptorProto {
        // ctx.enter(Definition::Message {
        //     full_name: self.full_name(&message.ast.name.value),
        // });

        // let name = s(&self.name.value);
        // let body = self.body.to_message_descriptor(ctx);

        // ctx.exit();
        // DescriptorProto { name, ..body }
        todo!()
    }

    fn check_enum(&self, e: &ast::Enum) -> EnumDescriptorProto {
        todo!()
    }

    fn check_extend(&self, e: &ast::Extend) -> FieldDescriptorProto {
        todo!()
    }

    fn check_service(&self, s: &ast::Service) -> ServiceDescriptorProto {
        todo!()
    }

    fn check_file_options(&self, options: &[ast::Option]) -> Option<FileOptions> {
        todo!()
    }
}
