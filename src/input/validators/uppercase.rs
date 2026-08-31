pub fn validate(value: &str) -> Option<String> {
    value
        .chars()
        .any(|character| character.is_lowercase())
        .then(|| "must be uppercase".to_owned())
}
