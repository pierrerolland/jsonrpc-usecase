pub fn validate<T, U>(value: &T, allowed: &[U]) -> Option<String>
where
    T: PartialEq<U>,
{
    (!allowed.iter().any(|candidate| value == candidate))
        .then(|| "must be one of the configured values".to_owned())
}
