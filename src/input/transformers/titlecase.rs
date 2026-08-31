pub fn transform(value: &mut String) {
    *value = value
        .split_inclusive(char::is_whitespace)
        .map(|part| {
            let mut characters = part.chars();
            let Some(first) = characters.next() else {
                return String::new();
            };
            first
                .to_uppercase()
                .chain(characters.flat_map(char::to_lowercase))
                .collect::<String>()
        })
        .collect();
}
