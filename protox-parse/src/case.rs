pub(crate) fn is_valid_ident(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_alphabetic()
        && s.as_bytes()[1..]
            .iter()
            .all(|&ch| ch.is_ascii_alphanumeric() || ch == b'_')
}

pub(crate) fn is_valid_group_name(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_uppercase()
        && s.as_bytes()[1..]
            .iter()
            .all(|&ch| ch.is_ascii_alphanumeric() || ch == b'_')
}

pub(crate) fn to_pascal_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut uppercase_next = true;

    for ch in name.chars() {
        if ch == '_' {
            uppercase_next = true
        } else if uppercase_next {
            result.push(ch.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}
