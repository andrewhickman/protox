use prost_types::{source_code_info::Location, SourceCodeInfo};

use super::ir;
use crate::lines::LineResolver;

impl<'a> ir::File<'a> {
    pub fn get_source_code_info(&self, source: &str) -> SourceCodeInfo {
        let ctx = Context {
            path: vec![],
            locations: vec![],
            lines: LineResolver::new(source),
        };

        // todo!();

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

// impl ast::Visitor for Context {
//     fn visit_file(&mut self, file: &ast::File) {
//         const NAME: i32 = 1;
//         const PACKAGE: i32 = 2;
//         const DEPENDENCY: i32 = 3;
//         const PUBLIC_DEPENDENCY: i32 = 10;
//         const WEAK_DEPENDENCY: i32 = 11;
//         const MESSAGE_TYPE: i32 = 4;
//         const ENUM_TYPE: i32 = 5;
//         const SERVICE: i32 = 6;
//         const EXTENSION: i32 = 7;
//         const OPTIONS: i32 = 8;
//         const SOURCE_CODE_INFO: i32 = 9;
//         const SYNTAX: i32 = 12;

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
//         const NAME: i32 = 1;
//         const VALUE: i32 = 2;
//         const OPTIONS: i32 = 3;
//         const RESERVED_RANGE: i32 = 4;
//         const RESERVED_NAME: i32 = 5;

//         enu.visit(self)
//     }

//     fn visit_enum_value(&mut self, _: &ast::EnumValue) {
//         const NAME: i32 = 1;
//         const NUMBER: i32 = 2;
//         const OPTIONS: i32 = 3;
//     }

//     fn visit_message(&mut self, message: &ast::Message) {
//         const NAME: i32 = 1;
//         const FIELD: i32 = 2;
//         const EXTENSION: i32 = 6;
//         const NESTED_TYPE: i32 = 3;
//         const ENUM_TYPE: i32 = 4;
//         const EXTENSION_RANGE: i32 = 5;
//         const OPTIONS: i32 = 7;
//         const ONEOF_DECL: i32 = 8;
//         const RESERVED_RANGE: i32 = 9;
//         const RESERVED_NAME: i32 = 10;

//         message.body.visit(self)
//     }

//     fn visit_message_field(&mut self, field: &ast::MessageField) {
//         field.visit(self)
//     }

//     fn visit_field(&mut self, _: &ast::Field) {
//         const NAME: i32 = 1;
//         const NUMBER: i32 = 3;
//         const LABEL: i32 = 4;
//         const TYPE: i32 = 5;
//         const TYPE_NAME: i32 = 6;
//         const EXTENDEE: i32 = 2;
//         const DEFAULT_VALUE: i32 = 7;
//         const ONEOF_INDEX: i32 = 9;
//         const JSON_NAME: i32 = 10;
//         const OPTIONS: i32 = 8;
//         const PROTO3_OPTIONAL: i32 = 17;
//     }

//     fn visit_map(&mut self, _: &ast::Map) {}

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
//         const NAME: i32 = 1;
//         const METHOD: i32 = 2;
//         const OPTIONS: i32 = 3;
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

// impl Context {
//     fn add_location(&mut self, span: Span) {
//         let span = self.lines.resolve_span(span);
//         self.locations.push(Location {
//             path: self.path.clone(),
//             span,
//             ..Default::default()
//         });
//     }

//     fn add_location_with_comments(&mut self, span: Span, comments: ast::Comments) {
//         let span = self.lines.resolve_span(span);
//         self.locations.push(Location {
//             path: self.path.clone(),
//             span,
//             leading_comments: comments.leading_comment,
//             trailing_comments: comments.trailing_comment,
//             leading_detached_comments: comments.leading_detached_comments,
//         });
//     }

//     fn with_path_item(&mut self, path_item: i32, f: impl FnOnce(&mut Self)) {
//         self.path.push(path_item);
//         f(self);
//         self.path.pop();
//     }

//     fn with_path_items<T>(
//         &mut self,
//         path_item: i32,
//         iter: impl IntoIterator<Item = T>,
//         mut f: impl FnMut(&mut Self, &T),
//     ) {
//         self.path.push(path_item);
//         for (index, item) in iter.into_iter().enumerate() {
//             self.path.push(index_to_i32(index));
//             f(self, &item);
//             self.path.pop();
//         }
//         self.path.pop();
//     }
// }
