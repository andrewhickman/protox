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
        assert_eq!(parser.peek(), None);
    };
    ($method:ident($source:expr) => $ast:expr) => {
        let mut parser = Parser::new($source);
        let result = parser.$method();
        assert_eq!(parser.lexer.extras.errors, vec![]);
        assert_eq!(result.unwrap(), $ast);
        assert_eq!(parser.peek(), None);
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
        comments: ast::Comments::default(),
    });
    case!(parse_option("//detached\n\n /*leading*/\noption foo = 5;//trailing") => ast::Option {
        name: ast::FullIdent::from(ast::Ident::new("foo", 32..35)),
        field_name: None,
        value: ast::Constant::Int(ast::Int {
            negative: false,
            value: 5,
            span: 38..39,
        }),
        comments: ast::Comments {
            leading_detached_comments: vec!["detached\n".to_owned()],
            leading_comment: Some("leading".to_owned()),
            trailing_comment: Some("trailing".to_owned()),
        },
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
        comments: ast::Comments::default(),
    });
    case!(parse_option("option (foo).bar = true;") => ast::Option {
        name: ast::FullIdent::from(ast::Ident::new("foo", 8..11)),
        field_name: Some(ast::FullIdent::from(ast::Ident::new("bar", 13..16))),
        value: ast::Constant::Bool(ast::Bool {
            value: true,
            span: 19..23,
        }),
        comments: ast::Comments::default(),
    });
    case!(parse_option("option ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier or '('".to_owned(),
        found: Token::Semicolon,
        span: 7..8,
    }]));
    case!(parse_option("option foo (") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or '='".to_owned(),
        found: Token::LeftParen,
        span: 11..12,
    }]));
    case!(parse_option("option foo.]") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::RightBracket,
        span: 11..12,
    }]));
    case!(parse_option("option foo = =") => Err(vec![ParseError::UnexpectedToken {
        expected: "a constant".to_owned(),
        found: Token::Equals,
        span: 13..14,
    }]));
    case!(parse_option("option foo = 1 )") => Err(vec![ParseError::UnexpectedToken {
        expected: "';'".to_owned(),
        found: Token::RightParen,
        span: 15..16,
    }]));
}

#[test]
fn parse_enum() {
    case!(parse_enum("enum Foo {}") => ast::Enum {
        name: ast::Ident::new("Foo", 5..8),
        values: vec![],
        options: vec![],
        reserved: vec![],
        comments: ast::Comments::default(),
    });
    case!(parse_enum("enum Foo { ; ; }") => ast::Enum {
        name: ast::Ident::new("Foo", 5..8),
        values: vec![],
        options: vec![],
        reserved: vec![],
        comments: ast::Comments::default(),
    });
    case!(parse_enum("/*detached*//*leading*/\nenum Foo {\n//trailing\n\n; ; }") => ast::Enum {
        name: ast::Ident::new("Foo", 29..32),
        values: vec![],
        options: vec![],
        reserved: vec![],
        comments: ast::Comments {
            leading_detached_comments: vec!["detached".to_owned()],
            leading_comment: Some("leading".to_owned()),
            trailing_comment: Some("trailing\n".to_owned()),
        },
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
            comments: ast::Comments::default(),
        }],
        options: vec![],
        reserved: vec![],
        comments: ast::Comments::default(),
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
            comments: ast::Comments::default(),
        }],
        options: vec![ast::Option {
            name: ast::FullIdent::from(ast::Ident::new("bar", 18..21)),
            field_name: None,
            value: ast::Constant::String(ast::String {
                value: "quz".to_owned(),
                span: 24..29
            }),
            comments: ast::Comments::default(),
        }],
        reserved: vec![],
        comments: ast::Comments::default(),
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
                comments: ast::Comments::default(),
            }],
            comments: ast::Comments::default(),
        }],
        options: vec![],
        reserved: vec![],
        comments: ast::Comments::default(),
    });
    case!(parse_enum("enum Foo { BAR = 0; reserved -1 to max; }") => ast::Enum {
        name: ast::Ident::new("Foo", 5..8),
        values: vec![ast::EnumValue {
            name: ast::Ident::new("BAR", 11..14),
            value: ast::Int {
                negative: false,
                value: 0,
                span: 17..18,
            },
            options: vec![],
            comments: ast::Comments::default(),
        }],
        options: vec![],
        reserved: vec![ast::Reserved::Ranges(vec![
            ast::ReservedRange {
                start: ast::Int {
                    negative: true,
                    value: 1,
                    span: 30..31,
                },
                end: ast::ReservedRangeEnd::Max,
            },
        ], ast::Comments::default())],
        comments: ast::Comments::default(),
    });
    case!(parse_enum("enum 3") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::IntLiteral(3),
        span: 5..6,
    }]));
    case!(parse_enum("enum Foo 0.1") => Err(vec![ParseError::UnexpectedToken {
        expected: "'{'".to_owned(),
        found: Token::FloatLiteral(0.1),
        span: 9..12,
    }]));
    case!(parse_enum("enum Foo {]") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier, '}', 'reserved' or 'option'".to_owned(),
        found: Token::RightBracket,
        span: 10..11,
    }]));
    case!(parse_enum("enum Foo { BAR .") => Err(vec![ParseError::UnexpectedToken {
        expected: "'='".to_owned(),
        found: Token::Dot,
        span: 15..16,
    }]));
    case!(parse_enum("enum Foo { BAR = foo") => Err(vec![ParseError::UnexpectedToken {
        expected: "an integer".to_owned(),
        found: Token::Ident("foo".into()),
        span: 17..20,
    }]));
}

#[test]
fn parse_service() {
    case!(parse_service("service Foo {}") => ast::Service {
        name: ast::Ident::new("Foo", 8..11),
        options: vec![],
        methods: vec![],
        comments: ast::Comments::default(),
    });
    case!(parse_service("service Foo { ; ; }") => ast::Service {
        name: ast::Ident::new("Foo", 8..11),
        options: vec![],
        methods: vec![],
        comments: ast::Comments::default(),
    });
    case!(parse_service("//detached\n\n//leading\nservice Foo {\n/* nottrailing */; ; }") => ast::Service {
        name: ast::Ident::new("Foo", 30..33),
        options: vec![],
        methods: vec![],
        comments: ast::Comments {
            leading_detached_comments: vec!["detached\n".to_owned()],
            leading_comment: Some("leading\n".to_owned()),
            trailing_comment: None,
        },
    });
    case!(parse_service("service service { }") => ast::Service {
        name: ast::Ident::new("service", 8..15),
        options: vec![],
        methods: vec![],
        comments: ast::Comments::default(),
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
            comments: ast::Comments::default(),
        }],
        comments: ast::Comments::default(),
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
            comments: ast::Comments::default(),
        }],
        comments: ast::Comments::default(),
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
            comments: ast::Comments::default(),
        }],
        comments: ast::Comments::default(),
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
            comments: ast::Comments::default(),
        }],
        comments: ast::Comments::default(),
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
                }),
                comments: ast::Comments::default(),
            }],
            comments: ast::Comments::default(),
        }],
        comments: ast::Comments::default(),
    });
    case!(parse_service("service ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Semicolon,
        span: 8..9,
    }]));
    case!(parse_service("service Foo (") => Err(vec![ParseError::UnexpectedToken {
        expected: "'{'".to_owned(),
        found: Token::LeftParen,
        span: 12..13,
    }]));
    case!(parse_service("service Foo { bar") => Err(vec![ParseError::UnexpectedToken {
        expected: "'rpc', '}', 'option' or ';'".to_owned(),
        found: Token::Ident("bar".into()),
        span: 14..17,
    }]));
    case!(parse_service("service Foo { rpc =") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Equals,
        span: 18..19,
    }]));
    case!(parse_service("service Foo { rpc bar{") => Err(vec![ParseError::UnexpectedToken {
        expected: "'('".to_owned(),
        found: Token::LeftBrace,
        span: 21..22,
    }]));
    case!(parse_service("service Foo { rpc bar(+") => Err(vec![ParseError::UnexpectedToken {
        expected: "'stream' or a type name".to_owned(),
        found: Token::Plus,
        span: 22..23,
    }]));
    case!(parse_service("service Foo { rpc bar(A(") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or ')'".to_owned(),
        found: Token::LeftParen,
        span: 23..24,
    }]));
    case!(parse_service("service Foo { rpc bar(A) [") => Err(vec![ParseError::UnexpectedToken {
        expected: "'returns'".to_owned(),
        found: Token::LeftBracket,
        span: 25..26,
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns =") => Err(vec![ParseError::UnexpectedToken {
        expected: "'('".to_owned(),
        found: Token::Equals,
        span: 33..34,
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns ()") => Err(vec![ParseError::UnexpectedToken {
        expected: "'stream' or a type name".to_owned(),
        found: Token::RightParen,
        span: 34..35,
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns (stream =)") => Err(vec![ParseError::UnexpectedToken {
        expected: "a type name".to_owned(),
        found: Token::Equals,
        span: 41..42,
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns (stream B}") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or ')'".to_owned(),
        found: Token::RightBrace,
        span: 42..43,
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns (stream B) )") => Err(vec![ParseError::UnexpectedToken {
        expected: "';' or '{'".to_owned(),
        found: Token::RightParen,
        span: 44..45,
    }]));
    case!(parse_service("service Foo { rpc bar(A) returns (stream B) {rpc") => Err(vec![ParseError::UnexpectedToken {
        expected: "'option', '}' or ';'".to_owned(),
        found: Token::Rpc,
        span: 45..48,
    }]));
}

#[test]
pub fn parse_package() {
    case!(parse_package("package foo;") => ast::Package {
        name: ast::FullIdent::from(ast::Ident::new("foo", 8..11)),
        comments: ast::Comments::default(),
    });
    case!(parse_package("//detached\n//detached2\n\n//detached3\n\npackage foo;\n/*trailing*/") => ast::Package {
        name: ast::FullIdent::from(ast::Ident::new("foo", 45..48)),
        comments: ast::Comments {
            leading_detached_comments: vec![
                "detached\ndetached2\n".to_owned(),
                "detached3\n".to_owned(),
            ],
            leading_comment: None,
            trailing_comment: Some("trailing".to_owned()),
        },
    });
    case!(parse_package("package foo.bar;") => ast::Package {
        name: ast::FullIdent::from(vec![
            ast::Ident::new("foo", 8..11),
            ast::Ident::new("bar", 12..15),
        ]),
        comments: ast::Comments::default(),
    });
    case!(parse_package("package =") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Equals,
        span: 8..9,
    }]));
    case!(parse_package("package foo)") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or ';'".to_owned(),
        found: Token::RightParen,
        span: 11..12,
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
        comments: ast::Comments::default(),
    });
    case!(parse_import("/*leading*/\nimport 'foo';/*trailing*/\n") => ast::Import {
        kind: None,
        value: ast::String {
            value: "foo".to_owned(),
            span: 19..24,
        },
        comments: ast::Comments {
            leading_detached_comments: vec![],
            leading_comment: Some("leading".to_owned()),
            trailing_comment: Some("trailing".to_owned()),
        },
    });
    case!(parse_import("import weak \"foo\";") => ast::Import {
        kind: Some(ast::ImportKind::Weak),
        value: ast::String {
            value: "foo".to_owned(),
            span: 12..17,
        },
        comments: ast::Comments::default(),
    });
    case!(parse_import("import public 'f\\x6fo';") => ast::Import {
        kind: Some(ast::ImportKind::Public),
        value: ast::String {
            value: "foo".to_owned(),
            span: 14..22,
        },
        comments: ast::Comments::default(),
    });
    case!(parse_import("import ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "a string literal, 'public' or 'weak'".to_owned(),
        found: Token::Semicolon,
        span: 7..8,
    }]));
    case!(parse_import("import public ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "a string literal".to_owned(),
        found: Token::Semicolon,
        span: 14..15,
    }]));
    case!(parse_import("import 'foo' message") => Err(vec![ParseError::UnexpectedToken {
        expected: "';'".to_owned(),
        found: Token::Message,
        span: 13..20,
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
        comments: ast::Comments::default(),
    });
    case!(parse_extension("/*leading*/extend Foo {\n//trailing\n }") => ast::Extension {
        extendee: ast::TypeName {
            leading_dot: None,
            name: ast::FullIdent::from(ast::Ident::new("Foo", 18..21)),
        },
        fields: vec![],
        comments: ast::Comments {
            leading_detached_comments: vec![],
            leading_comment: Some("leading".to_owned()),
            trailing_comment: Some("trailing\n".to_owned()),
        },
    });
    case!(parse_extension("extend Foo { ; ; }") => ast::Extension {
        extendee: ast::TypeName {
            leading_dot: None,
            name: ast::FullIdent::from(ast::Ident::new("Foo", 7..10)),
        },
        fields: vec![],
        comments: ast::Comments::default(),
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
                comments: ast::Comments::default(),
            }),
        ],
        comments: ast::Comments::default(),
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
                comments: ast::Comments::default(),
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
                comments: ast::Comments::default(),
            }),
        ],
        comments: ast::Comments::default(),
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
                            options: vec![],
                            comments: ast::Comments::default(),
                        })
                    ],
                    ..Default::default()
                },
                comments: ast::Comments::default(),
            }),
        ],
        comments: ast::Comments::default(),
    });
    case!(parse_extension("extend ] ") => Err(vec![ParseError::UnexpectedToken {
        expected: "a type name".to_owned(),
        found: Token::RightBracket,
        span: 7..8,
    }]));
    case!(parse_extension("extend Foo =") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or '{'".to_owned(),
        found: Token::Equals,
        span: 11..12,
    }]));
    case!(parse_extension("extend Foo { )") => Err(vec![ParseError::UnexpectedToken {
        expected: "a message field, '}' or ';'".to_owned(),
        found: Token::RightParen,
        span: 13..14,
    }]));
}

#[test]
pub fn parse_reserved() {
    case!(parse_reserved("//detached\n\nreserved 'foo';//trailing") => ast::Reserved::Names(vec![
        ast::Ident::new("foo", 21..26),
    ], ast::Comments {
        leading_detached_comments: vec!["detached\n".to_owned()],
        leading_comment: None,
        trailing_comment: Some("trailing".to_owned()),
    }));
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
    ], ast::Comments::default()));
    case!(parse_reserved("reserved -1;") => ast::Reserved::Ranges(vec![
        ast::ReservedRange {
            start: ast::Int { negative: true, value: 1, span: 10..11 },
            end: ast::ReservedRangeEnd::None,
        }
    ], ast::Comments::default()));
    case!(parse_reserved("reserved 'foo', 'bar';") => ast::Reserved::Names(vec![
        ast::Ident::new("foo", 9..14),
        ast::Ident::new("bar", 16..21),
    ], ast::Comments::default()));
    case!(parse_reserved("reserved ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "a positive integer or string".to_owned(),
        found: Token::Semicolon,
        span: 9..10,
    }]));
    case!(parse_reserved("reserved '0foo';") => ast::Reserved::Names(vec![
        ast::Ident::new("0foo", 9..15),
    ], ast::Comments::default()), Err(vec![ParseError::InvalidIdentifier {
        span: 9..15,
    }]));
}

#[test]
pub fn parse_group() {
    case!(parse_field("//leading\noptional group A = 1 {\n/*trailing*/ }") => ast::MessageField::Group(ast::Group {
        label: Some(ast::FieldLabel::Optional),
        name: ast::Ident::new("A", 25..26),
        number: ast::Int {
            negative: false,
            value: 1,
            span: 29..30,
        },
        body: ast::MessageBody::default(),
        comments: ast::Comments {
            leading_detached_comments: vec![],
            leading_comment: Some("leading\n".to_owned()),
            trailing_comment: Some("trailing".to_owned()),
        },
    }));
    case!(parse_field("optional group A = 1 { }") => ast::MessageField::Group(ast::Group {
        label: Some(ast::FieldLabel::Optional),
        name: ast::Ident::new("A", 15..16),
        number: ast::Int {
            negative: false,
            value: 1,
            span: 19..20,
        },
        body: ast::MessageBody::default(),
        comments: ast::Comments::default(),
    }));
    case!(parse_field("optional group A = 1 { ; ; }") => ast::MessageField::Group(ast::Group {
        label: Some(ast::FieldLabel::Optional),
        name: ast::Ident::new("A", 15..16),
        number: ast::Int {
            negative: false,
            value: 1,
            span: 19..20,
        },
        body: ast::MessageBody::default(),
        comments: ast::Comments::default(),
    }));
    case!(parse_field("optional group A = 1 { optional sint32 foo = 2; }") => ast::MessageField::Group(ast::Group {
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
                    options: vec![],
                    comments: ast::Comments::default(),
                })
            ],
            ..Default::default()
        },
        comments: ast::Comments::default(),
    }));
    case!(parse_field("optional group a = 1 { }") => ast::MessageField::Group(ast::Group {
        label: Some(ast::FieldLabel::Optional),
        name: ast::Ident::new("a", 15..16),
        number: ast::Int {
            negative: false,
            value: 1,
            span: 19..20,
        },
        body: ast::MessageBody::default(),
        comments: ast::Comments::default(),
    }), Err(vec![ParseError::InvalidGroupName {
        span: 15..16,
    }]));
    case!(parse_field("optional group , { }") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Comma,
        span: 15..16,
    }]));
    case!(parse_field("optional group a [") => Err(vec![
        ParseError::InvalidGroupName {
            span: 15..16,
        },
        ParseError::UnexpectedToken {
            expected: "'='".to_owned(),
            found: Token::LeftBracket,
            span: 17..18,
        },
    ]));
    case!(parse_field("optional group A = {") => Err(vec![ParseError::UnexpectedToken {
        expected: "a positive integer".to_owned(),
        found: Token::LeftBrace,
        span: 19..20,
    }]));
    case!(parse_field("optional group A = 1 ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "'{'".to_owned(),
        found: Token::Semicolon,
        span: 21..22,
    }]));
    case!(parse_field("optional group A = 1 {]") => Err(vec![ParseError::UnexpectedToken {
        expected: "a message field, oneof, reserved range, enum, message, option or '}'".to_owned(),
        found: Token::RightBracket,
        span: 22..23,
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
        comments: ast::Comments::default(),
    });
    case!(parse_map("/*leading*/map<string, int32> projects = 3;\n/*trailing*/\n") => ast::Map {
        key_ty: ast::KeyTy::String,
        ty: ast::Ty::Int32,
        name: ast::Ident::new("projects", 30..38),
        number: ast::Int {
            negative: false,
            value: 3,
            span: 41..42,
        },
        options: vec![],
        comments: ast::Comments {
            leading_detached_comments: vec![],
            leading_comment: Some("leading".to_string()),
            trailing_comment: Some("trailing".to_owned()),
        },
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
                comments: ast::Comments::default(),
            },
            ast::Option {
                name: ast::FullIdent::from(ast::Ident::new("opt2", 39..43)),
                field_name: None,
                value: ast::Constant::Float(ast::Float {
                    value: 4.5,
                    span: 46..49
                }),
                comments: ast::Comments::default(),
            },
        ],
        comments: ast::Comments::default(),
    });
    case!(parse_map("map>") => Err(vec![ParseError::UnexpectedToken {
        expected: "'<'".to_owned(),
        found: Token::RightAngleBracket,
        span: 3..4,
    }]));
    case!(parse_map("map<;") => Err(vec![ParseError::UnexpectedToken {
        expected: "an integer type or 'string'".to_owned(),
        found: Token::Semicolon,
        span: 4..5,
    }]));
    case!(parse_map("map<int32(") => Err(vec![ParseError::UnexpectedToken {
        expected: "','".to_owned(),
        found: Token::LeftParen,
        span: 9..10,
    }]));
    case!(parse_map("map<string, =") => Err(vec![ParseError::UnexpectedToken {
        expected: "a field type".to_owned(),
        found: Token::Equals,
        span: 12..13,
    }]));
    case!(parse_map("map<string, .Foo,") => Err(vec![ParseError::UnexpectedToken {
        expected: "'.' or '>'".to_owned(),
        found: Token::Comma,
        span: 16..17,
    }]));
    case!(parse_map("map<string, Foo> ;") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::Semicolon,
        span: 17..18,
    }]));
    case!(parse_map("map<string, Foo> foo ]") => Err(vec![ParseError::UnexpectedToken {
        expected: "'='".to_owned(),
        found: Token::RightBracket,
        span: 21..22,
    }]));
    case!(parse_map("map<string, Foo> foo = x") => Err(vec![ParseError::UnexpectedToken {
        expected: "a positive integer".to_owned(),
        found: Token::Ident("x".into()),
        span: 23..24,
    }]));
    case!(parse_map("map<string, Foo> foo = 1service") => Err(vec![ParseError::UnexpectedToken {
        expected: "';' or '['".to_owned(),
        found: Token::Service,
        span: 24..31,
    }]));
}

#[test]
pub fn parse_message() {
    case!(parse_message("message Foo {}") => ast::Message {
        name: ast::Ident::new("Foo", 8..11),
        body: ast::MessageBody::default(),
        comments: ast::Comments::default(),
    });
    case!(parse_message("//detached\n/*leading*/message Foo {/*trailing*/}") => ast::Message {
        name: ast::Ident::new("Foo", 30..33),
        body: ast::MessageBody::default(),
        comments: ast::Comments {
            leading_detached_comments: vec!["detached\n".to_owned()],
            leading_comment: Some("leading".to_owned()),
            trailing_comment: Some("trailing".to_owned()),
        },
    });
    case!(parse_message("message Foo { ; ; }") => ast::Message {
        name: ast::Ident::new("Foo", 8..11),
        body: ast::MessageBody::default(),
        comments: ast::Comments::default(),
    });
    case!(parse_message("message Foo {\
        message Bar {}\
        enum Quz {}\
        extend Bar {}\
    }") => ast::Message {
        name: ast::Ident::new("Foo", 8..11),
        body: ast::MessageBody {
            messages: vec![ast::Message {
                name: ast::Ident::new("Bar", 21..24),
                body: ast::MessageBody::default(),
                comments: ast::Comments::default(),
            }],
            enums: vec![ast::Enum {
                name: ast::Ident::new("Quz", 32..35),
                values: vec![],
                options: vec![],
                reserved: vec![],
                comments: ast::Comments::default(),
            }],
            extensions: vec![ast::Extension {
                extendee: ast::TypeName {
                    leading_dot: None,
                    name: ast::FullIdent::from(ast::Ident::new("Bar", 45..48)),
                },
                fields: vec![],
                comments: ast::Comments::default(),
            }],
            ..Default::default()
        },
        comments: ast::Comments::default(),
    });
    case!(parse_message("message Foo {
        fixed32 a = 1;
        map<int32, bool> b = 2;

        optional group C = 3 {
            required float d = 1;
        }

        oneof x {
            string y = 4;
        }
    }") => ast::Message {
        name: ast::Ident::new("Foo", 8..11),
        body: ast::MessageBody {
            fields: vec![
                ast::MessageField::Field(ast::Field {
                    label: None,
                    name: ast::Ident::new("a", 30..31),
                    ty: ast::Ty::Fixed32,
                    number: ast::Int {
                        negative: false,
                        value: 1,
                        span: 34..35
                    },
                    options: vec![],
                    comments: ast::Comments::default(),
                }),
                ast::MessageField::Map(ast::Map {
                    key_ty: ast::KeyTy::Int32,
                    ty: ast::Ty::Bool,
                    name: ast::Ident::new("b", 62..63),
                    number: ast::Int {
                        negative: false,
                        value: 2,
                        span: 66..67,
                    },
                    options: vec![],
                    comments: ast::Comments::default(),
                }),
                ast::MessageField::Group(ast::Group {
                    label: Some(ast::FieldLabel::Optional),
                    name: ast::Ident::new("C", 93..94),
                    number: ast::Int {
                        negative: false,
                        value: 3,
                        span: 97..98,
                    },
                    body: ast::MessageBody {
                        fields: vec![
                            ast::MessageField::Field(ast::Field {
                                label: Some(ast::FieldLabel::Required),
                                name: ast::Ident::new("d", 128..129),
                                ty: ast::Ty::Float,
                                number: ast::Int {
                                    negative: false,
                                    value: 1,
                                    span: 132..133,
                                },
                                options: vec![],
                                comments: ast::Comments::default(),
                            })
                        ],
                        ..Default::default()
                    },
                    comments: ast::Comments::default(),
                }),
            ],
            oneofs: vec![ast::Oneof {
                name: ast::Ident::new("x", 160..161),
                options: vec![],
                fields: vec![ast::MessageField::Field(ast::Field {
                    label: None,
                    name: ast::Ident::new("y", 183..184),
                    ty: ast::Ty::String,
                    number: ast::Int {
                        negative: false,
                        value: 4,
                        span: 187..188,
                    },
                    options: vec![],
                    comments: ast::Comments::default(),
                })],
                comments: ast::Comments::default(),
            }],
            ..Default::default()
        },
        comments: ast::Comments::default(),
    });
    case!(parse_message("message Foo { , }") => Err(vec![ParseError::UnexpectedToken {
        expected: "a message field, oneof, reserved range, enum, message, option or '}'".to_owned(),
        found: Token::Comma,
        span: 14..15,
    }]));
}

#[test]
pub fn parse_oneof() {
    case!(parse_oneof("oneof Foo {}") => ast::Oneof {
        name: ast::Ident::new("Foo", 6..9),
        fields: vec![],
        options: vec![],
        comments: ast::Comments::default(),
    });
    case!(parse_oneof("oneof Foo { ; ; }") => ast::Oneof {
        name: ast::Ident::new("Foo", 6..9),
        fields: vec![],
        options: vec![],
        comments: ast::Comments::default(),
    });
    case!(parse_oneof("/*detached1*///detached2\n\n//leading\noneof Foo {/*nottrailing*/ ; ; }") => ast::Oneof {
        name: ast::Ident::new("Foo", 42..45),
        fields: vec![],
        options: vec![],
        comments: ast::Comments {
            leading_detached_comments: vec!["detached1".to_owned(), "detached2\n".to_owned()],
            leading_comment: Some("leading\n".to_owned()),
            trailing_comment: None,
        },
    });
    case!(parse_oneof("oneof Foo { int32 bar = 1; }") => ast::Oneof {
        name: ast::Ident::new("Foo", 6..9),
        fields: vec![ast::MessageField::Field(ast::Field {
            label: None,
            ty: ast::Ty::Int32,
            name: ast::Ident::new("bar", 18..21),
            number: ast::Int {
                negative: false,
                value: 1,
                span: 24..25,
            },
            options: vec![],
            comments: ast::Comments::default(),
        })],
        options: vec![],
        comments: ast::Comments::default(),
    });
    case!(parse_oneof("oneof 10.4") => Err(vec![ParseError::UnexpectedToken {
        expected: "an identifier".to_owned(),
        found: Token::FloatLiteral(10.4),
        span: 6..10,
    }]));
    case!(parse_oneof("oneof Foo <") => Err(vec![ParseError::UnexpectedToken {
        expected: "'{'".to_owned(),
        found: Token::LeftAngleBracket,
        span: 10..11,
    }]));
    case!(parse_oneof("oneof Foo { ,") => Err(vec![ParseError::UnexpectedToken {
        expected: "a message field, option or '}'".to_owned(),
        found: Token::Comma,
        span: 12..13,
    }]));
    case!(parse_oneof("oneof Foo { bytes b = 1 }") => Err(vec![ParseError::UnexpectedToken {
        expected: "';' or '['".to_owned(),
        found: Token::RightBrace,
        span: 24..25,
    }]));
}

#[test]
pub fn parse_file() {
    case!(parse_file("") => ast::File {
        syntax: ast::Syntax::Proto2,
        packages: vec![],
        imports: vec![],
        options: vec![],
        definitions: vec![],
    });
    case!(parse_file("
        package protox.lib;
    ") => ast::File {
        syntax: ast::Syntax::Proto2,
        packages: vec![ast::Package {
            name: ast::FullIdent::from(vec![
                ast::Ident::new("protox", 17..23),
                ast::Ident::new("lib", 24..27),
            ]),
            comments: ast::Comments::default(),
        }],
        imports: vec![],
        options: vec![],
        definitions: vec![],
    });
    case!(parse_file("
        syntax = 'proto2';

        option optimize_for = SPEED;
    ") => ast::File {
        syntax: ast::Syntax::Proto2,
        packages: vec![],
        imports: vec![],
        definitions: vec![],
        options: vec![ast::Option {
            name: ast::FullIdent::from(ast::Ident::new("optimize_for", 44..56)),
            field_name: None,
            value: ast::Constant::FullIdent(ast::FullIdent::from(ast::Ident::new("SPEED", 59..64))),
            comments: ast::Comments::default(),
        }],
    });
    case!(parse_file("
        syntax = \"proto3\";

        import \"foo.proto\";
    ") => ast::File {
        syntax: ast::Syntax::Proto3,
        packages: vec![],
        imports: vec![ast::Import {
            kind: None,
            value: ast::String {
                value: "foo.proto".to_owned(),
                span: 44..55,
            },
            comments: ast::Comments::default(),
        }],
        definitions: vec![],
        options: vec![],
    });
    case!(parse_file("
        syntax = 'unknown';
    ") => Err(vec![ParseError::UnknownSyntax {
        span: 18..27,
    }]));
    case!(parse_file("
        syntax = 'proto2';

        message Foo { , }
        enum Bar { ; }
        option quz 1;
    ") => ast::File {
        syntax: ast::Syntax::Proto2,
        packages: vec![],
        imports: vec![],
        definitions: vec![ast::Definition::Enum(ast::Enum {
            name: ast::Ident::new("Bar", 68..71),
            values: vec![],
            options: vec![],
            reserved: vec![],
            comments: ast::Comments::default(),
        })],
        options: vec![],
    }, Err(vec![
        ParseError::UnexpectedToken {
            expected: "a message field, oneof, reserved range, enum, message, option or '}'".to_string(),
            found: Token::Comma,
            span: 51..52,
        },
        ParseError::UnexpectedToken {
            expected: "'.' or '='".to_string(),
            found: Token::IntLiteral(1),
            span: 97..98,
        },
    ]));
    case!(parse_file("
        syntax = 'proto3';

        message Foo {
            // trailing

            // detached

            // leading
            int32 bar = 1;
            // trailing2
        }
    ") => ast::File {
        syntax: ast::Syntax::Proto3,
        packages: vec![],
        imports: vec![],
        definitions: vec![ast::Definition::Message(ast::Message {
            name: ast::Ident::new("Foo", 45..48),
            body: ast::MessageBody {
                fields: vec![ast::MessageField::Field(ast::Field {
                    label: None,
                    name: ast::Ident::new("bar", 142..145),
                    ty: ast::Ty::Int32,
                    number: ast::Int {
                        negative: false,
                        value: 1,
                        span: 148..149,
                    },
                    options: vec![],
                    comments: ast::Comments {
                        leading_detached_comments: vec![" detached\n".to_owned()],
                        leading_comment: Some(" leading\n".to_owned()),
                        trailing_comment: Some(" trailing2\n".to_owned()),
                    },
                })],
                ..Default::default()
            },
            comments: ast::Comments {
                leading_detached_comments: vec![],
                leading_comment: None,
                trailing_comment: Some(" trailing\n".to_owned()),
            },
        })],
        options: vec![],
    });
}
