pub fn validate(value: &str) -> Option<String> {
    (!value.is_ascii()).then(|| "must contain only ASCII characters".to_owned())
}
