use super::length::Length;

pub fn validate<T: Length + ?Sized>(value: &T) -> Option<String> {
    (value.length() == 0).then(|| "must not be empty".to_owned())
}
