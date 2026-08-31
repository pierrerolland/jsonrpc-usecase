pub fn validate<T: PartialOrd>(value: &T, minimum: &T) -> Option<String> {
    (value < minimum).then(|| "must be greater than or equal to the minimum".to_owned())
}
