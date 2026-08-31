use super::number::Number;

pub fn validate<T: Number>(value: &T) -> Option<String> {
    (*value > T::zero()).then(|| "must be non-positive".to_owned())
}
