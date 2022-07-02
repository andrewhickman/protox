pub fn to_camel_case(name: &str) -> String {
    // todo!()
    name.to_owned()
}

pub fn to_pascal_case(name: &str) -> String {
    // todo!()
    name.to_owned()
}

pub fn to_lower_without_underscores(name: &str) -> String {
    name.chars()
        .filter_map(|ch| match ch {
            '_' => None,
            _ => Some(ch.to_ascii_lowercase()),
        })
        .collect()
}
