pub fn validate(value: &str) -> Option<String> {
    value
        .trim()
        .is_empty()
        .then(|| "must not be blank".to_owned())
}
