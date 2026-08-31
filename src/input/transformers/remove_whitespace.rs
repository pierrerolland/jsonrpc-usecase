pub fn transform(value: &mut String) {
    value.retain(|character| !character.is_whitespace());
}
