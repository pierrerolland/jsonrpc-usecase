pub fn transform(value: &mut String, suffix: &str) {
    if let Some(stripped) = value.strip_suffix(suffix) {
        *value = stripped.to_owned();
    }
}
