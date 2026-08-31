pub fn validate(value: &str) -> Option<String> {
    let value = value.strip_suffix('.').unwrap_or(value);
    let valid = !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        });

    (!valid).then(|| "must be a valid ASCII hostname".to_owned())
}
