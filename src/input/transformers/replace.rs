pub fn transform(value: &mut String, from: &str, to: &str) {
    *value = value.replace(from, to);
}
