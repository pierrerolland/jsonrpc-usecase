pub fn transform(value: &mut String) {
    *value = value.split_whitespace().collect::<Vec<_>>().join(" ");
}
