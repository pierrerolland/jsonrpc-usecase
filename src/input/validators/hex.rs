pub fn validate(value: &str) -> Option<String> {
    let valid = !value.is_empty()
        && value.len().is_multiple_of(2)
        && value.bytes().all(|byte| byte.is_ascii_hexdigit());

    (!valid).then(|| "must be an even-length hexadecimal string".to_owned())
}
