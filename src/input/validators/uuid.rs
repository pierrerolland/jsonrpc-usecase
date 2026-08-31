pub fn validate(value: &str) -> Option<String> {
    uuid::Uuid::parse_str(value)
        .is_err()
        .then(|| "must be a valid UUID".to_owned())
}
