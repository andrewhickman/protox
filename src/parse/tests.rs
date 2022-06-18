use super::*;

macro_rules! case {
    ($method:ident($source:expr) => Err($errors:expr)) => {
        let mut parser = Parser::new($source);
        parser.$method().unwrap_err();
        assert_eq!(parser.lexer.extras.errors, $errors);
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
        found: Token::Int(3),
        span: SourceSpan::from(5..6),
    }]));
    case!(parse_enum("enum Foo 0.1") => Err(vec![ParseError::UnexpectedToken {
        expected: "'{'".to_owned(),
        found: Token::Float(0.1),
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
