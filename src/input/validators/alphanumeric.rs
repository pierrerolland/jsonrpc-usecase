pub fn validate(value: &str) -> Option<String> {
    (value.is_empty() || !value.chars().all(char::is_alphanumeric))
        .then(|| "must contain only alphanumeric characters".to_owned())
}
