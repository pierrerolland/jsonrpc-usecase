pub fn validate(value: &str) -> Option<String> {
    let valid = !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');

    (!valid).then(|| "must be a lowercase URL slug".to_owned())
}
