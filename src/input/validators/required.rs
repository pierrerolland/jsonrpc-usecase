pub fn validate<T>(value: &Option<T>) -> Option<String> {
    value.is_none().then(|| "is required".to_owned())
}
