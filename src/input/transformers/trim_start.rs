pub fn transform(value: &mut String) {
    let trimmed = value.trim_start();
    if trimmed.len() != value.len() {
        *value = trimmed.to_owned();
    }
}
