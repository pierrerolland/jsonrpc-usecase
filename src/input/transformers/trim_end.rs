pub fn transform(value: &mut String) {
    let trimmed = value.trim_end();
    if trimmed.len() != value.len() {
        *value = trimmed.to_owned();
    }
}
