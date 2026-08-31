use base64::{Engine as _, engine::general_purpose::STANDARD};

pub fn validate(value: &str) -> Option<String> {
    STANDARD
        .decode(value)
        .is_err()
        .then(|| "must use standard Base64 encoding".to_owned())
}
