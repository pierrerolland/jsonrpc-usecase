pub fn transform(value: &mut String, prefix: &str) {
    if let Some(stripped) = value.strip_prefix(prefix) {
        *value = stripped.to_owned();
    }
}
