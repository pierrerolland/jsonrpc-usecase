pub fn validate(value: &str, expected: &str) -> Option<String> {
    (!value.ends_with(expected)).then(|| format!("must end with {expected:?}"))
}
