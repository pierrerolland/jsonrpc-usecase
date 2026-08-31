pub fn validate(value: &str, expected: &str) -> Option<String> {
    (!value.starts_with(expected)).then(|| format!("must start with {expected:?}"))
}
