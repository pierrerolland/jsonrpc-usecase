pub fn transform(value: &mut String) {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return;
    };

    *value = first.to_uppercase().chain(characters).collect();
}
