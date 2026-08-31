pub fn validate(value: &str) -> Option<String> {
    (value.is_empty() || !value.chars().all(char::is_alphabetic))
        .then(|| "must contain only alphabetic characters".to_owned())
}
