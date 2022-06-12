#[derive(Logos)]
enum Component<'a> {
    #[regex(r#"[^\x00\n\\'"]+"#)]
    Unescaped(&'a str),
    #[regex(r#"['"]"#, terminator)]
    Terminator(char),
    #[regex(r#"\\[xX][0-9A-Fa-f][0-9A-Fa-f]"#, hex_escape)]
    #[regex(r#"\\[0-7][0-7][0-7]"#, oct_escape)]
    #[regex(r#"\\[abfnrtv\\'"]"#, char_escape)]
    Char(char),
    #[error]
    Error,
}