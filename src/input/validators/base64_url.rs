use base64::{
    Engine as _,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};

pub fn validate(value: &str) -> Option<String> {
    (URL_SAFE.decode(value).is_err() && URL_SAFE_NO_PAD.decode(value).is_err())
        .then(|| "must use URL-safe Base64 encoding".to_owned())
}
