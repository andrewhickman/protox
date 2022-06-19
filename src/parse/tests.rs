use super::*;

macro_rules! case {
    ($method:ident($source:expr) => Err($errors:expr)) => {
        let mut parser = Parser::new($source);
        parser.$method().unwrap_err();
        assert_eq!(parser.lexer.extras.errors, $errors);
    };
    ($method:ident($source:expr) => $ast:expr, Err($errors:expr)) => {
        let mut parser = Parser::new($source);
        let result = parser.$method();
        assert_eq!(parser.lexer.extras.errors, $errors);
        assert_eq!(result.unwrap(), $ast);
    };
    ($method:ident($source:expr) => $ast:expr) => {
        let mut parser = Parser::new($source);
        let result = parser.$method();
        assert_eq!(parser.lexer.extras.errors, vec![]);
        assert_eq!(result.unwrap(), $ast);
    };
}

#[test]
pub fn parse_option() {
    case!(parse_option("option foo = 5;") => ast::Option {
        name: ast::FullIdent::from(ast::Ident::new("foo", 7..10)),
        field_name: None,
        value: ast::Constant::Int(ast::Int {
            negative: false,
            value: 5,
            span: 13..14,
        }),
    });
    case!(parse_option("option (foo.bar) = \"hello\";") => ast::Option {
        name: ast::FullIdent::from(vec![
            ast::Ident::new("foo", 8..11),
            ast::Ident::new("bar", 12..15),
        ]),
        field_name: None,
        value: ast::Constant::String(ast::String {
            value: "hello".to_string(),
            span: 19..26,
        }),
    });
    case!(parse_option("option (foo).bar = true;") => ast::Option {
        name: ast::FullIdent::from(ast::Ident::new("foo", 8..11)),
        field_name: Some(ast::FullIdent::from(ast::Ident::new("bar", 13..16))),
        value: ast::Constant::Bool(ast::Bool {
            value: true,
            span: 19..23,
        }),
    });
    case!(parse_option("option ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier or '('".to_owned(),
        found: Token::Semicolon,
        span: SourceSpan::from(7..8),
    }]));
    case!(parse_option("option foo (") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or '='".to_owned(),
        found: Token::LeftParen,
        span: SourceSpan::from(11..12),
    }]));
    case!(parse_option("option foo.]") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::RightBracket,
        span: SourceSpan::from(11..12),
    }]));
    case!(parse_option("option foo = =") => Err(vec![ParseError::UnexpectedToken {
        expected: "a constant".to_owned(),
        found: Token::Equals,
        span: SourceSpan::from(13..14),
    }]));
    case!(parse_option("option foo = 1 )") => Err(vec![ParseError::UnexpectedToken {
        expected: "';'".to_owned(),
        found: Token::RightParen,
        span: SourceSpan::from(15..16),
    }]));
}

#[test]
fn parse_enum() {
    case!(parse_enum("enum Foo {}") => ast::Enum {
        name: ast::Ident::new("Foo", 5..8),
        values: vec![],
        options: vec![],
    });
    case!(parse_enum("enum Foo { ; ; }") => ast::Enum {
        name: ast::Ident::new("Foo", 5..8),
        values: vec![],
        options: vec![],
    });
    case!(parse_enum("enum Foo { BAR = 1; }") => ast::Enum {
        name: ast::Ident::new("Foo", 5..8),
        values: vec![ast::EnumValue {
            name: ast::Ident::new("BAR", 11..14),
            value: ast::Int {
                negative: false,
                value: 1,
                span: 17..18,
            },
            options: vec![],
        }],
        options: vec![],
    });
    case!(parse_enum("enum Foo { option bar = 'quz' ; VAL = -1; }") => ast::Enum {
        name: ast::Ident::new("Foo", 5..8),
        values: vec![ast::EnumValue {
            name: ast::Ident::new("VAL", 32..35),
            value: ast::Int {
                negative: true,
                value: 1,
                span: 39..40,
            },
            options: vec![],
        }],
        options: vec![ast::Option {
            name: ast::FullIdent::from(ast::Ident::new("bar", 18..21)),
            field_name: None,
            value: ast::Constant::String(ast::String {
                value: "quz".to_owned(),
                span: 24..29
            }),
        }],
    });
    case!(parse_enum("enum Foo { BAR = 0 [opt = 0.5]; }") => ast::Enum {
        name: ast::Ident::new("Foo", 5..8),
        values: vec![ast::EnumValue {
            name: ast::Ident::new("BAR", 11..14),
            value: ast::Int {
                negative: false,
                value: 0,
                span: 17..18,
            },
            options: vec![ast::Option {
                name: ast::FullIdent::from(ast::Ident::new("opt", 20..23)),
                field_name: None,
                value: ast::Constant::Float(ast::Float {
                    value: 0.5,
                    span: 26..29
                }),
            }],
        }],
        options: vec![],
    });
    case!(parse_enum("enum 3") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::IntLiteral(3),
        span: SourceSpan::from(5..6),
    }]));
    case!(parse_enum("enum Foo 0.1") => Err(vec![ParseError::UnexpectedToken {
        expected: "'{'".to_owned(),
        found: Token::FloatLiteral(0.1),
        span: SourceSpan::from(9..12),
    }]));
    case!(parse_enum("enum Foo {]") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier, '}', ';' or 'option'".to_owned(),
        found: Token::RightBracket,
        span: SourceSpan::from(10..11),
    }]));
    case!(parse_enum("enum Foo { BAR .") => Err(vec![ParseError::UnexpectedToken {
        expected: "'='".to_owned(),
        found: Token::Dot,
        span: SourceSpan::from(15..16),
    }]));
    case!(parse_enum("enum Foo { BAR = foo") => Err(vec![ParseError::UnexpectedToken {
        expected: "an integer".to_owned(),
        found: Token::Ident("foo".to_owned()),
        span: SourceSpan::from(17..20),
    }]));
}

#[test]
fn parse_service() {
    case!(parse_service("service Foo {}") => ast::Service {
        name: ast::Ident::new("Foo", 8..11),
        options: vec![],
        methods: vec![],
    });
    case!(parse_service("service Foo { ; ; }") => ast::Service {
        name: ast::Ident::new("Foo", 8..11),
        options: vec![],
        methods: vec![],
    });
    case!(parse_service("service service { }") => ast::Service {
        name: ast::Ident::new("service", 8..15),
        options: vec![],
        methods: vec![],
    });
    case!(parse_service("service Foo { rpc bar(A) returns (B.C); }") => ast::Service {
        name: ast::Ident::new("Foo", 8..11),
        options: vec![],
        methods: vec![ast::Method {
            name: ast::Ident::new("bar", 18..21),
            is_client_streaming: false,
            input_ty: ast::TypeName {
                leading_dot: None,
                name: FullIdent::from(ast::Ident::new("A", 22..23)),
            },
            is_server_streaming: false,
            output_ty: ast::TypeName {
                leading_dot: None,
                name: FullIdent::from(vec![
                    ast::Ident::new("B", 34..35),
                    ast::Ident::new("C", 36..37),
                ]),
            },
            options: vec![],
        }],
    });
    case!(parse_service("service Foo { rpc bar(stream .A.B) returns (stream .C); }") => ast::Service {
        name: ast::Ident::new("Foo", 8..11),
        options: vec![],
        methods: vec![ast::Method {
            name: ast::Ident::new("bar", 18..21),
            is_client_streaming: true,
            input_ty: ast::TypeName {
                leading_dot: Some(29..30),
                name: FullIdent::from(vec![
                    ast::Ident::new("A", 30..31),
                    ast::Ident::new("B", 32..33),
                ]),
            },
            is_server_streaming: true,
            output_ty: ast::TypeName {
                leading_dot: Some(51..52),
                name: FullIdent::from(ast::Ident::new("C", 52..53)),
            },
            options: vec![],
        }],
    });
    case!(parse_service("service Foo { rpc bar(A) returns (B.C) { } }") => ast::Service {
        name: ast::Ident::new("Foo", 8..11),
        options: vec![],
        methods: vec![ast::Method {
            name: ast::Ident::new("bar", 18..21),
            is_client_streaming: false,
            input_ty: ast::TypeName {
                leading_dot: None,
                name: FullIdent::from(ast::Ident::new("A", 22..23)),
            },
            is_server_streaming: false,
            output_ty: ast::TypeName {
                leading_dot: None,
                name: FullIdent::from(vec![
                    ast::Ident::new("B", 34..35),
                    ast::Ident::new("C", 36..37),
                ]),
            },
            options: vec![],
        }],
    });
    case!(parse_service("service Foo { rpc bar(A) returns (B.C) { ; ; } }") => ast::Service {
        name: ast::Ident::new("Foo", 8..11),
        options: vec![],
        methods: vec![ast::Method {
            name: ast::Ident::new("bar", 18..21),
            is_client_streaming: false,
            input_ty: ast::TypeName {
                leading_dot: None,
                name: FullIdent::from(ast::Ident::new("A", 22..23)),
            },
            is_server_streaming: false,
            output_ty: ast::TypeName {
                leading_dot: None,
                name: FullIdent::from(vec![
                    ast::Ident::new("B", 34..35),
                    ast::Ident::new("C", 36..37),
                ]),
            },
            options: vec![],
        }],
    });
    case!(parse_service("service Foo { rpc bar(A) returns (B.C) { option opt = -1; } }") => ast::Service {
        name: ast::Ident::new("Foo", 8..11),
        options: vec![],
        methods: vec![ast::Method {
            name: ast::Ident::new("bar", 18..21),
            is_client_streaming: false,
            input_ty: ast::TypeName {
                leading_dot: None,
                name: FullIdent::from(ast::Ident::new("A", 22..23)),
            },
            is_server_streaming: false,
            output_ty: ast::TypeName {
                leading_dot: None,
                name: FullIdent::from(vec![
                    ast::Ident::new("B", 34..35),
                    ast::Ident::new("C", 36..37),
                ]),
            },
            options: vec![ast::Option {
                name: ast::FullIdent::from(ast::Ident::new("opt", 48..51)),
                field_name: None,
                value: ast::Constant::Int(ast::Int {
                    negative: true,
                    value: 1,
                    span: 55..56,
                })
            }],
        }],
    });
    case!(parse_service("service ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Semicolon,
        span: SourceSpan::from(8..9),
    }]));
    case!(parse_service("service Foo (") => Err(vec![ParseError::UnexpectedToken {
        expected: "'{'".to_owned(),
        found: Token::LeftParen,
        span: SourceSpan::from(12..13),
    }]));
    case!(parse_service("service Foo { bar") => Err(vec![ParseError::UnexpectedToken {
        expected: "'rpc', '}', 'option' or ';'".to_owned(),
        found: Token::Ident("bar".to_owned()),
        span: SourceSpan::from(14..17),
    }]));
    case!(parse_service("service Foo { rpc =") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Equals,
        span: SourceSpan::from(18..19),
    }]));
    case!(parse_service("service Foo { rpc bar{") => Err(vec![ParseError::UnexpectedToken {
        expected: "'('".to_owned(),
        found: Token::LeftBrace,
        span: SourceSpan::from(21..22),
    }]));
    case!(parse_service("service Foo { rpc bar(+") => Err(vec![ParseError::UnexpectedToken {
        expected: "'stream' or a type name".to_owned(),
        found: Token::Plus,
        span: SourceSpan::from(22..23),
    }]));
    case!(parse_service("service Foo { rpc bar(A(") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or ')'".to_owned(),
        found: Token::LeftParen,
        span: SourceSpan::from(23..24),
    }]));
    case!(parse_service("service Foo { rpc bar(A) [") => Err(vec![ParseError::UnexpectedToken {
        expected: "'returns'".to_owned(),
        found: Token::LeftBracket,
        span: SourceSpan::from(25..26),
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns =") => Err(vec![ParseError::UnexpectedToken {
        expected: "'('".to_owned(),
        found: Token::Equals,
        span: SourceSpan::from(33..34),
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns ()") => Err(vec![ParseError::UnexpectedToken {
        expected: "'stream' or a type name".to_owned(),
        found: Token::RightParen,
        span: SourceSpan::from(34..35),
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns (stream =)") => Err(vec![ParseError::UnexpectedToken {
        expected: "a type name".to_owned(),
        found: Token::Equals,
        span: SourceSpan::from(41..42),
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns (stream B}") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or ')'".to_owned(),
        found: Token::RightBrace,
        span: SourceSpan::from(42..43),
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns (stream B) )") => Err(vec![ParseError::UnexpectedToken {
        expected: "';' or '{'".to_owned(),
        found: Token::RightParen,
        span: SourceSpan::from(44..45),
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns (stream B) {rpc") => Err(vec![ParseError::UnexpectedToken {
        expected: "'option', '}' or ';'".to_owned(),
        found: Token::Rpc,
        span: SourceSpan::from(45..48),
    }]));
}

#[test]
pub fn parse_package() {
    case!(parse_package("package foo;") => ast::Package {
        name: ast::FullIdent::from(ast::Ident::new("foo", 8..11)),
    });
    case!(parse_package("package foo.bar;") => ast::Package {
        name: ast::FullIdent::from(vec![
            ast::Ident::new("foo", 8..11),
            ast::Ident::new("bar", 12..15),
        ]),
    });
    case!(parse_package("package =") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Equals,
        span: SourceSpan::from(8..9),
    }]));
    case!(parse_package("package foo)") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or ';'".to_owned(),
        found: Token::RightParen,
        span: SourceSpan::from(11..12),
    }]));
}

#[test]
pub fn parse_import() {
    case!(parse_import("import 'foo';") => ast::Import {
        kind: None,
        value: ast::String {
            value: "foo".to_owned(),
            span: 7..12,
        },
    });
    case!(parse_import("import weak \"foo\";") => ast::Import {
        kind: Some(ast::ImportKind::Weak),
        value: ast::String {
            value: "foo".to_owned(),
            span: 12..17,
        },
    });
    case!(parse_import("import public 'f\\x6fo';") => ast::Import {
        kind: Some(ast::ImportKind::Public),
        value: ast::String {
            value: "foo".to_owned(),
            span: 14..22,
        },
    });
    case!(parse_import("import ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "a string literal, 'public' or 'weak'".to_owned(),
        found: Token::Semicolon,
        span: SourceSpan::from(7..8),
    }]));
    case!(parse_import("import public ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "a string literal".to_owned(),
        found: Token::Semicolon,
        span: SourceSpan::from(14..15),
    }]));
}

#[test]
pub fn parse_extension() {
    case!(parse_extension("extend Foo { }") => ast::Extension {
        extendee: ast::TypeName {
            leading_dot: None,
            name: ast::FullIdent::from(ast::Ident::new("Foo", 7..10)),
        },
        fields: vec![],
    });
    case!(parse_extension("extend Foo { ; ; }") => ast::Extension {
        extendee: ast::TypeName {
            leading_dot: None,
            name: ast::FullIdent::from(ast::Ident::new("Foo", 7..10)),
        },
        fields: vec![],
    });
    case!(parse_extension("extend Foo.Foo { optional int32 bar = 126; }") => ast::Extension {
        extendee: ast::TypeName {
            leading_dot: None,
            name: ast::FullIdent::from(vec![
                ast::Ident::new("Foo", 7..10),
                ast::Ident::new("Foo", 11..14),
            ]),
        },
        fields: vec![
            ast::MessageField::Field(ast::Field {
                label: Some(ast::FieldLabel::Optional),
                ty: ast::Ty::Int32,
                name: ast::Ident::new("bar", 32..35),
                number: ast::Int {
                    negative: false,
                    value: 126,
                    span: 38..41,
                },
                options: vec![],
            }),
        ],
    });
    case!(parse_extension("extend .Foo { optional int32 bar = 126; repeated string quz = 127; }") => ast::Extension {
        extendee: ast::TypeName {
            leading_dot: Some(7..8),
            name: ast::FullIdent::from(ast::Ident::new("Foo", 8..11)),
        },
        fields: vec![
            ast::MessageField::Field(ast::Field {
                label: Some(ast::FieldLabel::Optional),
                ty: ast::Ty::Int32,
                name: ast::Ident::new("bar", 29..32),
                number: ast::Int {
                    negative: false,
                    value: 126,
                    span: 35..38,
                },
                options: vec![],
            }),
            ast::MessageField::Field(ast::Field {
                label: Some(ast::FieldLabel::Repeated),
                ty: ast::Ty::String,
                name: ast::Ident::new("quz", 56..59),
                number: ast::Int {
                    negative: false,
                    value: 127,
                    span: 62..65,
                },
                options: vec![],
            }),
        ],
    });
    case!(parse_extension("extend Foo { repeated group A = 1 { optional string name = 2; } }") => ast::Extension {
        extendee: ast::TypeName {
            leading_dot: None,
            name: ast::FullIdent::from(ast::Ident::new("Foo", 7..10)),
        },
        fields: vec![
            ast::MessageField::Group(ast::Group {
                label: Some(ast::FieldLabel::Repeated),
                name: ast::Ident::new("A", 28..29),
                number: ast::Int {
                    negative: false,
                    value: 1,
                    span: 32..33,
                },
                body: ast::MessageBody {
                    fields: vec![
                        ast::MessageField::Field(ast::Field {
                            label: Some(ast::FieldLabel::Optional),
                            name: ast::Ident::new("name", 52..56),
                            ty: ast::Ty::String,
                            number: ast::Int {
                                negative: false,
                                value: 2,
                                span: 59..60
                            },
                            options: vec![]
                        })
                    ],
                    ..Default::default()
                }
            }),
        ],
    });
    case!(parse_extension("extend ] ") => Err(vec![ParseError::UnexpectedToken {
        expected: "a type name".to_owned(),
        found: Token::RightBracket,
        span: SourceSpan::from(7..8),
    }]));
    case!(parse_extension("extend Foo =") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or '{'".to_owned(),
        found: Token::Equals,
        span: SourceSpan::from(11..12),
    }]));
    case!(parse_extension("extend Foo { )") => Err(vec![ParseError::UnexpectedToken {
        expected: "a message field, '}' or ';'".to_owned(),
        found: Token::RightParen,
        span: SourceSpan::from(13..14),
    }]));
}

#[test]
pub fn parse_reserved() {
    case!(parse_reserved("reserved 0, 2 to 3, 5 to max;") => ast::Reserved::Ranges(vec![
        ast::ReservedRange {
            start: ast::Int { negative: false, value: 0, span: 9..10 },
            end: ast::ReservedRangeEnd::None,
        },
        ast::ReservedRange {
            start: ast::Int { negative: false, value: 2, span: 12..13 },
            end: ast::ReservedRangeEnd::Int(ast::Int {
                negative: false, value: 3, span: 17..18
            }),
        },
        ast::ReservedRange {
            start: ast::Int { negative: false, value: 5, span: 20..21 },
            end: ast::ReservedRangeEnd::Max,
        },
    ]));
    case!(parse_reserved("reserved 'foo', 'bar';") => ast::Reserved::Names(vec![
        ast::Ident::new("foo", 9..14),
        ast::Ident::new("bar", 16..21),
    ]));
    case!(parse_reserved("reserved -1;") => Err(vec![ParseError::UnexpectedToken {
        expected: "a positive integer or string".to_owned(),
        found: Token::Minus,
        span: SourceSpan::from(9..10),
    }]));
    case!(parse_reserved("reserved ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "a positive integer or string".to_owned(),
        found: Token::Semicolon,
        span: SourceSpan::from(9..10),
    }]));
    case!(parse_reserved("reserved '0foo';") => ast::Reserved::Names(vec![
        ast::Ident::new("0foo", 9..15),
    ]), Err(vec![ParseError::InvalidIdentifier {
        span: SourceSpan::from(9..15),
    }]));
}

#[test]
#[ignore]
pub fn parse_field() {
    todo!()
}

#[test]
pub fn parse_group() {
    case!(parse_field("optional group A = 1 { } }") => ast::MessageField::Group(ast::Group {
        label: Some(ast::FieldLabel::Optional),
        name: ast::Ident::new("A", 15..16),
        number: ast::Int {
            negative: false,
            value: 1,
            span: 19..20,
        },
        body: ast::MessageBody::default(),
    }));
    case!(parse_field("optional group A = 1 { ; ; } }") => ast::MessageField::Group(ast::Group {
        label: Some(ast::FieldLabel::Optional),
        name: ast::Ident::new("A", 15..16),
        number: ast::Int {
            negative: false,
            value: 1,
            span: 19..20,
        },
        body: ast::MessageBody::default(),
    }));
    case!(parse_field("optional group A = 1 { optional sint32 foo = 2; } }") => ast::MessageField::Group(ast::Group {
        label: Some(ast::FieldLabel::Optional),
        name: ast::Ident::new("A", 15..16),
        number: ast::Int {
            negative: false,
            value: 1,
            span: 19..20,
        },
        body: ast::MessageBody {
            fields: vec![
                ast::MessageField::Field(ast::Field {
                    label: Some(ast::FieldLabel::Optional),
                    name: ast::Ident::new("foo", 39..42),
                    ty: ast::Ty::Sint32,
                    number: ast::Int {
                        negative: false,
                        value: 2,
                        span: 45..46
                    },
                    options: vec![]
                })
            ],
            ..Default::default()
        }
    }));
    case!(parse_field("optional group a = 1 { } }") => ast::MessageField::Group(ast::Group {
        label: Some(ast::FieldLabel::Optional),
        name: ast::Ident::new("a", 15..16),
        number: ast::Int {
            negative: false,
            value: 1,
            span: 19..20,
        },
        body: ast::MessageBody::default(),
    }), Err(vec![ParseError::InvalidGroupName {
        span: SourceSpan::from(15..16),
    }]));
    case!(parse_field("optional group , { } }") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Comma,
        span: SourceSpan::from(15..16),
    }]));
    case!(parse_field("optional group a [") => Err(vec![
        ParseError::InvalidGroupName {
            span: SourceSpan::from(15..16),
        },
        ParseError::UnexpectedToken {
            expected: "'='".to_owned(),
            found: Token::LeftBracket,
            span: SourceSpan::from(17..18),
        },
    ]));
    case!(parse_field("optional group A = {") => Err(vec![ParseError::UnexpectedToken {
        expected: "a positive integer".to_owned(),
        found: Token::LeftBrace,
        span: SourceSpan::from(19..20),
    }]));
    case!(parse_field("optional group A = 1 ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "'{'".to_owned(),
        found: Token::Semicolon,
        span: SourceSpan::from(21..22),
    }]));
    case!(parse_field("optional group A = 1 {]") => Err(vec![ParseError::UnexpectedToken {
        expected: "a message field, oneof, reserved range, enum, message or '}'".to_owned(),
        found: Token::RightBracket,
        span: SourceSpan::from(22..23),
    }]));
}

#[test]
pub fn parse_map() {
    case!(parse_map("map<string, Project> projects = 3;") => ast::Map {
        key_ty: ast::KeyTy::String,
        ty: ast::Ty::Named(ast::TypeName {
            leading_dot: None,
            name: ast::FullIdent::from(ast::Ident::new("Project", 12..19)),
        }),
        name: ast::Ident::new("projects", 21..29),
        number: ast::Int {
            negative: false,
            value: 3,
            span: 32..33,
        },
        options: vec![],
    });
    case!(parse_map("map<int32, bool> name = 5 [opt = true, opt2 = 4.5];") => ast::Map {
        key_ty: ast::KeyTy::Int32,
        ty: ast::Ty::Bool,
        name: ast::Ident::new("name", 17..21),
        number: ast::Int {
            negative: false,
            value: 5,
            span: 24..25,
        },
        options: vec![
            ast::Option {
                name: ast::FullIdent::from(ast::Ident::new("opt", 27..30)),
                field_name: None,
                value: ast::Constant::Bool(ast::Bool {
                    value: true,
                    span: 33..37
                }),
            },
            ast::Option {
                name: ast::FullIdent::from(ast::Ident::new("opt2", 39..43)),
                field_name: None,
                value: ast::Constant::Float(ast::Float {
                    value: 4.5,
                    span: 46..49
                }),
            },
        ],
    });
    case!(parse_map("map>") => Err(vec![ParseError::UnexpectedToken {
        expected: "'<'".to_owned(),
        found: Token::RightAngleBracket,
        span: SourceSpan::from(3..4),
    }]));
    case!(parse_map("map<;") => Err(vec![ParseError::UnexpectedToken {
        expected: "an integer type or 'string'".to_owned(),
        found: Token::Semicolon,
        span: SourceSpan::from(4..5),
    }]));
    case!(parse_map("map<int32(") => Err(vec![ParseError::UnexpectedToken {
        expected: "','".to_owned(),
        found: Token::LeftParen,
        span: SourceSpan::from(9..10),
    }]));
    case!(parse_map("map<string, =") => Err(vec![ParseError::UnexpectedToken {
        expected: "a field type".to_owned(),
        found: Token::Equals,
        span: SourceSpan::from(12..13),
    }]));
    case!(parse_map("map<string, .Foo,") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or '>'".to_owned(),
        found: Token::Comma,
        span: SourceSpan::from(16..17),
    }]));
    case!(parse_map("map<string, Foo> ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Semicolon,
        span: SourceSpan::from(17..18),
    }]));
    case!(parse_map("map<string, Foo> foo ]") => Err(vec![ParseError::UnexpectedToken {
        expected: "'='".to_owned(),
        found: Token::RightBracket,
        span: SourceSpan::from(21..22),
    }]));
    case!(parse_map("map<string, Foo> foo = x") => Err(vec![ParseError::UnexpectedToken {
        expected: "a positive integer".to_owned(),
        found: Token::Ident("x".to_string()),
        span: SourceSpan::from(23..24),
    }]));
    case!(parse_map("map<string, Foo> foo = 1service") => Err(vec![ParseError::UnexpectedToken {
        expected: "';' or '['".to_owned(),
        found: Token::Service,
        span: SourceSpan::from(24..31),
    }]));
}

#[test]
#[ignore]
pub fn parse_message() {
    todo!()
}

#[test]
#[ignore]
pub fn parse_file() {
    // TODO error recovery
    todo!()
}
