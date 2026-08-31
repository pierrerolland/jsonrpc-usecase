pub fn validate(value: &str) -> Option<String> {
    value
        .chars()
        .any(|character| character.is_uppercase())
        .then(|| "must be lowercase".to_owned())
}
