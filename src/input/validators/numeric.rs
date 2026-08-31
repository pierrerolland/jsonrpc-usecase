pub fn validate(value: &str) -> Option<String> {
    (value.is_empty() || !value.chars().all(char::is_numeric))
        .then(|| "must contain only numeric characters".to_owned())
}
