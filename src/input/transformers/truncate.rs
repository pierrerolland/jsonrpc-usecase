pub fn transform(value: &mut String, maximum: usize) {
    let Some((byte_index, _)) = value.char_indices().nth(maximum) else {
        return;
    };
    value.truncate(byte_index);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_on_character_boundaries() {
        let mut value = "éclair".to_owned();
        transform(&mut value, 2);
        assert_eq!(value, "éc");
    }
}
