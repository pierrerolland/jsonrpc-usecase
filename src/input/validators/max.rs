pub fn validate<T: PartialOrd>(value: &T, maximum: &T) -> Option<String> {
    (value > maximum).then(|| "must be less than or equal to the maximum".to_owned())
}
