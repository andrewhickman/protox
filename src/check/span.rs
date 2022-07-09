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

        ctx.locations.sort_by_key(|loc| (loc.span[0], loc.span[1]));

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

        self.with_path_item(OPTIONS, |ctx| ctx.visit_options(&file.ast.options));

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
        self.with_path_items(ONEOF_DECL, message.oneofs.iter(), |ctx, oneof| {
            ctx.visit_oneof(oneof);
        });

        self.with_path_items(NESTED_TYPE, message.messages.iter(), |ctx, message| {
            ctx.visit_message(message);
        });
        self.with_path_items(ENUM_TYPE, body.enums(), |ctx, enu| {
            ctx.visit_enum(enu);
        });

        self.with_path_item(OPTIONS, |ctx| ctx.visit_options(&body.options));

        self.with_path_item(EXTENSION_RANGE, |ctx| {
            ctx.visit_extensions(body.extensions.iter());
        });
        self.with_path_item(RESERVED_RANGE, |ctx| {
            ctx.visit_reserved_ranges(body.reserved_ranges());
        });
        self.with_path_item(RESERVED_NAME, |ctx| {
            ctx.visit_reserved_names(body.reserved_names());
        });
    }

    fn visit_field(&mut self, field: &ir::Field) {
        const NAME: i32 = 1;
        const NUMBER: i32 = 3;
        const LABEL: i32 = 4;
        const TYPE: i32 = 5;
        const TYPE_NAME: i32 = 6;
        const DEFAULT_VALUE: i32 = 7;
        const OPTIONS: i32 = 8;

        let ast = match &field.ast {
            ir::FieldSource::Field(ast) => ast,
            ir::FieldSource::MapKey(..) | ir::FieldSource::MapValue(..) => return,
        };

        self.add_location_with_comments(ast.span.clone(), ast.comments.clone());

        self.with_path_item(NAME, |ctx| ctx.add_location(ast.name.span.clone()));
        self.with_path_item(NUMBER, |ctx| ctx.add_location(ast.number.span.clone()));

        if let Some((_, label_span)) = &ast.label {
            self.with_path_item(LABEL, |ctx| ctx.add_location(label_span.clone()));
        }

        match &ast.kind {
            ast::FieldKind::Normal {
                ty: ast::Ty::Named(name),
                ..
            } => {
                self.with_path_item(TYPE_NAME, |ctx| ctx.add_location(name.span()));
            }
            ast::FieldKind::Normal { ty_span, .. } => {
                self.with_path_item(TYPE, |ctx| ctx.add_location(ty_span.clone()));
            }
            ast::FieldKind::Group { ty_span, .. } => {
                self.with_path_item(TYPE, |ctx| ctx.add_location(ty_span.clone()));
                self.with_path_item(TYPE_NAME, |ctx| ctx.add_location(ast.name.span.clone()));
            }
            ast::FieldKind::Map { ty_span, .. } => {
                self.with_path_item(TYPE_NAME, |ctx| ctx.add_location(ty_span.clone()));
            }
        }

        if let Some(default_value) = &ast.default_value() {
            self.with_path_item(DEFAULT_VALUE, |ctx| {
                ctx.add_location(default_value.value.span());
            });
        }

        if let Some(options) = &ast.options {
            self.with_path_item(OPTIONS, |ctx| {
                ctx.visit_options_list(options);
            });
        }
    }

    fn visit_extensions<'a>(&mut self, extensions: impl Iterator<Item = &'a ast::Extensions>) {
        const START: i32 = 1;
        const END: i32 = 2;
        const OPTIONS: i32 = 3;

        let mut count = 0;
        for extension in extensions {
            self.add_location_with_comments(extension.span.clone(), extension.comments.clone());

            for range in &extension.ranges {
                self.with_path_item(count, |ctx| {
                    ctx.add_location(range.span());
                    ctx.with_path_item(START, |ctx| {
                        ctx.add_location(range.start_span());
                    });
                    ctx.with_path_item(END, |ctx| {
                        ctx.add_location(range.end_span());
                    });
                    if let Some(options) = &extension.options {
                        ctx.with_path_item(OPTIONS, |ctx| {
                            ctx.visit_options_list(options);
                        });
                    }
                });
                count += 1;
            }
        }
    }

    fn visit_oneof(&mut self, oneof: &ir::Oneof) {
        const NAME: i32 = 1;
        const OPTIONS: i32 = 2;

        if let ir::OneofSource::Oneof(oneof) = &oneof.ast {
            self.add_location(oneof.span.clone());
            self.with_path_item(NAME, |ctx| {
                ctx.add_location(oneof.name.span.clone());
            });
            self.with_path_item(OPTIONS, |ctx| {
                ctx.visit_options(&oneof.options);
            });
        }
    }

    fn visit_enum(&mut self, enu: &ast::Enum) {
        const NAME: i32 = 1;
        const VALUE: i32 = 2;
        const OPTIONS: i32 = 3;
        const RESERVED_RANGE: i32 = 4;
        const RESERVED_NAME: i32 = 5;

        self.add_location_with_comments(enu.span.clone(), enu.comments.clone());
        self.with_path_item(NAME, |ctx| {
            ctx.add_location(enu.name.span.clone());
        });
        self.with_path_items(VALUE, enu.values.iter(), |ctx, value| {
            ctx.visit_enum_value(value);
        });
        self.with_path_item(OPTIONS, |ctx| {
            ctx.visit_options(&enu.options);
        });
        self.with_path_item(RESERVED_RANGE, |ctx| {
            ctx.visit_reserved_ranges(enu.reserved_ranges());
        });
        self.with_path_item(RESERVED_NAME, |ctx| {
            ctx.visit_reserved_names(enu.reserved_names());
        });
    }

    fn visit_enum_value(&mut self, value: &ast::EnumValue) {
        const NAME: i32 = 1;
        const NUMBER: i32 = 2;
        const OPTIONS: i32 = 3;

        self.add_location_with_comments(value.span.clone(), value.comments.clone());
        self.with_path_item(NAME, |ctx| {
            ctx.add_location(value.span.clone());
        });
        self.with_path_item(NUMBER, |ctx| {
            ctx.add_location(value.number.span.clone());
        });
        if let Some(options) = &value.options {
            self.with_path_item(OPTIONS, |ctx| {
                ctx.visit_options_list(options);
            });
        }
    }

    fn visit_reserved_ranges<'a>(
        &mut self,
        reserveds: impl Iterator<Item = (&'a ast::Reserved, &'a [ast::ReservedRange])>,
    ) {
        const START: i32 = 1;
        const END: i32 = 2;

        let mut count = 0;
        for (reserved, ranges) in reserveds {
            self.add_location_with_comments(reserved.span.clone(), reserved.comments.clone());

            for range in ranges {
                self.with_path_item(count, |ctx| {
                    ctx.add_location(range.span());
                    ctx.with_path_item(START, |ctx| {
                        ctx.add_location(range.start_span());
                    });
                    ctx.with_path_item(END, |ctx| {
                        ctx.add_location(range.end_span());
                    });
                });
                count += 1;
            }
        }
    }

    fn visit_reserved_names<'a>(
        &mut self,
        reserveds: impl Iterator<Item = (&'a ast::Reserved, &'a [ast::Ident])>,
    ) {
        let mut count = 0;
        for (reserved, names) in reserveds {
            self.add_location_with_comments(reserved.span.clone(), reserved.comments.clone());

            for name in names {
                self.with_path_item(count, |ctx| {
                    ctx.add_location(name.span.clone());
                });
                count += 1;
            }
        }
    }

    fn visit_service(&mut self, service: &ast::Service) {
        const NAME: i32 = 1;
        const METHOD: i32 = 2;
        const OPTIONS: i32 = 3;

        self.add_location_with_comments(service.span.clone(), service.comments.clone());
        self.with_path_item(NAME, |ctx| {
            ctx.add_location(service.name.span.clone());
        });
        self.with_path_items(METHOD, service.methods.iter(), |ctx, method| {
            ctx.visit_method(method);
        });
        self.with_path_item(OPTIONS, |ctx| {
            ctx.visit_options(&service.options);
        });
    }

    fn visit_method(&mut self, method: &ast::Method) {
        const NAME: i32 = 1;
        const INPUT_TYPE: i32 = 2;
        const OUTPUT_TYPE: i32 = 3;
        const OPTIONS: i32 = 4;
        const CLIENT_STREAMING: i32 = 5;
        const SERVER_STREAMING: i32 = 6;

        self.add_location_with_comments(method.span.clone(), method.comments.clone());
        self.with_path_item(NAME, |ctx| {
            ctx.add_location(method.name.span.clone());
        });
        self.with_path_item(INPUT_TYPE, |ctx| {
            ctx.add_location(method.input_ty.span());
        });
        self.with_path_item(OUTPUT_TYPE, |ctx| {
            ctx.add_location(method.output_ty.span());
        });
        if let Some(span) = &method.client_streaming {
            self.with_path_item(CLIENT_STREAMING, |ctx| {
                ctx.add_location(span.clone());
            });
        }
        if let Some(span) = &method.server_streaming {
            self.with_path_item(SERVER_STREAMING, |ctx| {
                ctx.add_location(span.clone());
            });
        }
        self.with_path_item(OPTIONS, |ctx| {
            ctx.visit_options(&method.options);
        });
    }

    fn visit_extend(&mut self, extend: &ast::Extend) {
        todo!();

        const FIELD_EXTENDEE: i32 = 2;
        // extendee for all types
    }

    fn visit_options(&mut self, options: &[ast::Option]) {
        for option in options {
            self.add_location(option.span.clone());
            // self.with_path_item(option.number, f)
        }
        // self.with_path_items(path_item, options, |ctx, option| {
        // self.add_location(option.span.clone());
        // });
    }

    fn visit_options_list(&mut self, options: &ast::OptionList) {
        self.add_location(options.span.clone());
        // self.with_path_items(path_item, &options.options, |ctx, option| {
        //     self.add_location(option.span.clone());
        // });
    }

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
