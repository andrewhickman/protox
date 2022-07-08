use logos::Span;
use prost_types::{source_code_info::Location, SourceCodeInfo};

use super::ir;
use crate::{ast, index_to_i32, lines::LineResolver};

impl<'a> ir::File<'a> {
    pub fn get_source_code_info(&self, source: &str) -> SourceCodeInfo {
        let mut ctx = Context {
            path: vec![],
            locations: vec![],
            lines: LineResolver::new(source),
        };

        ctx.visit_file(self);

        ctx.locations.sort_by_key(|loc| loc.span.first().copied());

        SourceCodeInfo {
            location: ctx.locations,
        }
    }
}

struct Context {
    pub path: Vec<i32>,
    pub locations: Vec<Location>,
    pub lines: LineResolver,
}

impl Context {
    fn visit_file(&mut self, file: &ir::File) {
        const PACKAGE: i32 = 2;
        const DEPENDENCY: i32 = 3;
        const PUBLIC_DEPENDENCY: i32 = 10;
        const WEAK_DEPENDENCY: i32 = 11;
        const MESSAGE_TYPE: i32 = 4;
        const ENUM_TYPE: i32 = 5;
        const SERVICE: i32 = 6;
        const EXTENSION: i32 = 7;
        const OPTIONS: i32 = 8;
        const SOURCE_CODE_INFO: i32 = 9;
        const SYNTAX: i32 = 12;

        self.add_location(file.ast.span.clone());

        if let Some(package) = &file.ast.package {
            self.with_path_item(PACKAGE, |ctx| {
                ctx.add_location_with_comments(package.span.clone(), package.comments.clone());
            });
        }

        self.with_path_items(DEPENDENCY, file.ast.imports.iter(), |ctx, import| {
            ctx.add_location_with_comments(import.span.clone(), import.comments.clone());
        });

        self.with_path_items(
            PUBLIC_DEPENDENCY,
            file.ast.public_imports(),
            |ctx, (_, import)| {
                ctx.add_location(import.span.clone());
            },
        );

        self.with_path_items(
            WEAK_DEPENDENCY,
            file.ast.weak_imports(),
            |ctx, (_, import)| {
                ctx.add_location(import.span.clone());
            },
        );

        self.with_path_items(MESSAGE_TYPE, file.messages.iter(), |ctx, message| {
            ctx.visit_message(message);
        });

        self.with_path_items(ENUM_TYPE, file.ast.enums(), |ctx, enu| {
            ctx.visit_enum(enu);
        });

        self.with_path_items(SERVICE, file.ast.services(), |ctx, service| {
            ctx.visit_service(service);
        });

        self.with_path_items(EXTENSION, file.ast.extends(), |ctx, extend| {
            ctx.visit_extend(extend);
        });

        self.visit_options(OPTIONS, &file.ast.options);

        if let Some(syntax_span) = &file.ast.syntax_span {
            self.with_path_item(SYNTAX, |ctx| {
                ctx.add_location(syntax_span.clone());
            });
        }
    }

    fn visit_message(&mut self, message: &ir::Message) {
        const NAME: i32 = 1;
        const FIELD: i32 = 2;
        const EXTENSION: i32 = 6;
        const NESTED_TYPE: i32 = 3;
        const ENUM_TYPE: i32 = 4;
        const EXTENSION_RANGE: i32 = 5;
        const OPTIONS: i32 = 7;
        const ONEOF_DECL: i32 = 8;
        const RESERVED_RANGE: i32 = 9;
        const RESERVED_NAME: i32 = 10;

        let body = match message.ast {
            ir::MessageSource::Message(message) => {
                self.add_location_with_comments(message.span.clone(), message.comments.clone());
                self.with_path_item(NAME, |ctx| {
                    ctx.add_location(message.name.span.clone());
                });
                &message.body
            }
            ir::MessageSource::Group(field, body) => {
                self.add_location_with_comments(field.span.clone(), field.comments.clone());
                self.with_path_item(NAME, |ctx| {
                    ctx.add_location(field.name.span.clone());
                });
                body
            }
            ir::MessageSource::Map(_) => return,
        };

        self.with_path_items(FIELD, message.fields.iter(), |ctx, field| {
            ctx.visit_field(field);
        });

        self.with_path_items(EXTENSION, body.extends(), |ctx, extend| {
            ctx.visit_extend(extend);
        });

        self.with_path_items(NESTED_TYPE, message.messages.iter(), |ctx, message| {
            ctx.visit_message(message);
        });

        self.with_path_items(ENUM_TYPE, body.enums(), |ctx, enu| {
            ctx.visit_enum(enu);
        });

        self.with_path_items(
            EXTENSION_RANGE,
            body.extensions.iter(),
            |ctx, extensions| {
                ctx.visit_extensions(extensions);
            },
        );

        self.visit_options(OPTIONS, &body.options);

        self.with_path_items(ONEOF_DECL, message.oneofs.iter(), |ctx, oneof| {
            ctx.visit_oneof(oneof);
        });

        self.with_path_items(
            RESERVED_RANGE,
            body.reserved_ranges(),
            |ctx, (reserved, range)| {
                ctx.visit_reserved_range(reserved, range);
            },
        );

        self.with_path_items(
            RESERVED_NAME,
            body.reserved_names(),
            |ctx, (reserved, name)| {
                ctx.visit_reserved_name(reserved, name);
            },
        );
    }

    fn visit_field(&mut self, field: &ir::Field) {
        const NAME: i32 = 1;
        const NUMBER: i32 = 3;
        const LABEL: i32 = 4;
        const TYPE: i32 = 5;
        const TYPE_NAME: i32 = 6;
        const EXTENDEE: i32 = 2;
        const DEFAULT_VALUE: i32 = 7;
        const ONEOF_INDEX: i32 = 9;
        const JSON_NAME: i32 = 10;
        const OPTIONS: i32 = 8;
        const PROTO3_OPTIONAL: i32 = 17;

        todo!()
    }

    fn visit_extensions(&mut self, extensions: &ast::Extensions) {
        todo!()
    }

    fn visit_oneof(&mut self, oneof: &ir::Oneof) {
        todo!()
    }

    fn visit_enum(&mut self, enu: &ast::Enum) {
        const NAME: i32 = 1;
        const VALUE: i32 = 2;
        const OPTIONS: i32 = 3;
        const RESERVED_RANGE: i32 = 4;
        const RESERVED_NAME: i32 = 5;

        todo!()
    }

    fn visit_reserved_range(&mut self, reserved: &ast::Reserved, range: &ast::ReservedRange) {
        todo!()
    }

    fn visit_reserved_name(&mut self, reserved: &ast::Reserved, name: &ast::Ident) {
        todo!()
    }

    fn visit_service(&mut self, service: &ast::Service) {
        const NAME: i32 = 1;
        const METHOD: i32 = 2;
        const OPTIONS: i32 = 3;

        todo!()
    }

    fn visit_extend(&mut self, extend: &ast::Extend) {
        todo!()
    }

    fn visit_options(&mut self, path_item: i32, options: &[ast::Option]) {
        todo!()
    }

    // impl ast::Visitor for Context {
    //     fn visit_file(&mut self, file: &ast::File) {

    //         self.add_location(file.span.clone());

    //         if let Some(package) = &file.package {
    //             self.with_path_item(PACKAGE, |this| {
    //                 this.add_location_with_comments(package.span.clone(), package.comments.clone())
    //             });
    //         }

    //         self.with_path_items(DEPENDENCY, &file.imports, |this, import| {
    //             this.add_location_with_comments(import.span.clone(), import.comments.clone());
    //         });

    //         // TODO add public /weak imports

    //         file.visit(self)
    //     }

    //     fn visit_enum(&mut self, enu: &ast::Enum) {

    //         enu.visit(self)
    //     }

    //     fn visit_enum_value(&mut self, _: &ast::EnumValue) {
    //         const NAME: i32 = 1;
    //         const NUMBER: i32 = 2;
    //         const OPTIONS: i32 = 3;
    //     }

    //     fn visit_message(&mut self, message: &ast::Message) {
    //         message.body.visit(self)
    //     }

    //     fn visit_message_field(&mut self, field: &ast::MessageField) {
    //         field.visit(self)
    //     }

    //     fn visit_field(&mut self, _: &ast::Field) {
    //     }

    //     fn visit_map(&mut self, _: &ast::Field) {}

    //     fn visit_group(&mut self, group: &ast::Group) {
    //         group.body.visit(self)
    //     }

    //     fn visit_oneof(&mut self, oneof: &ast::Oneof) {
    //         const NAME: i32 = 1;
    //         const OPTIONS: i32 = 2;
    //         oneof.visit(self)
    //     }

    //     fn visit_extend(&mut self, extend: &ast::Extend) {
    //         extend.visit(self)
    //     }

    //     fn visit_service(&mut self, service: &ast::Service) {
    //         service.visit(self)
    //     }

    //     fn visit_method(&mut self, _: &ast::Method) {
    //         const NAME: i32 = 1;
    //         const INPUT_TYPE: i32 = 2;
    //         const OUTPUT_TYPE: i32 = 3;
    //         const OPTIONS: i32 = 4;
    //         const CLIENT_STREAMING: i32 = 5;
    //         const SERVER_STREAMING: i32 = 6;
    //     }
    // }

    fn add_location(&mut self, span: Span) {
        let span = self.lines.resolve_span(span);
        self.locations.push(Location {
            path: self.path.clone(),
            span,
            ..Default::default()
        });
    }

    fn add_location_with_comments(&mut self, span: Span, comments: ast::Comments) {
        let span = self.lines.resolve_span(span);
        self.locations.push(Location {
            path: self.path.clone(),
            span,
            leading_comments: comments.leading_comment,
            trailing_comments: comments.trailing_comment,
            leading_detached_comments: comments.leading_detached_comments,
        });
    }

    fn with_path_item(&mut self, path_item: i32, f: impl FnOnce(&mut Self)) {
        self.path.push(path_item);
        f(self);
        self.path.pop();
    }

    fn with_path_items<T>(
        &mut self,
        path_item: i32,
        iter: impl IntoIterator<Item = T>,
        mut f: impl FnMut(&mut Self, T),
    ) {
        self.path.push(path_item);
        for (index, item) in iter.into_iter().enumerate() {
            self.path.push(index_to_i32(index));
            f(self, item);
            self.path.pop();
        }
        self.path.pop();
    }
}
