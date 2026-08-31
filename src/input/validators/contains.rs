pub fn validate(value: &str, expected: &str) -> Option<String> {
    (!value.contains(expected)).then(|| format!("must contain {expected:?}"))
}
