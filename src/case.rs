pub fn to_json_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut uppercase_next = false;

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

// TODO is this required?
#[allow(unused)]
pub fn to_camel_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut uppercase_next = false;

    let mut name = name.chars();
    if let Some(ch) = name.next() {
        result.push(ch.to_ascii_lowercase());
    }

    for ch in name {
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

pub fn to_pascal_case(name: &str) -> String {
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

pub fn to_lower_without_underscores(name: &str) -> String {
    name.chars()
        .filter_map(|ch| match ch {
            '_' => None,
            _ => Some(ch.to_ascii_lowercase()),
        })
        .collect()
}
