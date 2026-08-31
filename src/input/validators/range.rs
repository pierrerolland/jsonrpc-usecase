pub fn validate<T: PartialOrd>(value: &T, minimum: &T, maximum: &T) -> Option<String> {
    (value < minimum || value > maximum).then(|| "must be within the inclusive range".to_owned())
}
