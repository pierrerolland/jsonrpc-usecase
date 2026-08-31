pub fn validate(value: &str) -> Option<String> {
    url::Url::parse(value)
        .is_err()
        .then(|| "must be a valid absolute URL".to_owned())
}
