pub fn validate<T, U>(value: &T, disallowed: U) -> Option<String>
where
    T: PartialEq<U>,
{
    (value == &disallowed).then(|| "must not equal the configured value".to_owned())
}
