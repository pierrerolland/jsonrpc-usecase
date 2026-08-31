pub fn validate<T, U>(value: &T, expected: U) -> Option<String>
where
    T: PartialEq<U>,
{
    (value != &expected).then(|| "must equal the configured value".to_owned())
}
