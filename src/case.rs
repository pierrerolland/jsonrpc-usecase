use serde_json::{Map, Value};

pub(crate) fn params_to_rust_case(value: Value) -> Value {
    map_object_keys(value, camel_to_snake)
}

pub(crate) fn value_to_json_case(value: Value) -> Value {
    map_object_keys(value, snake_to_camel)
}

fn map_object_keys(value: Value, convert: fn(&str) -> String) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| map_object_keys(item, convert))
                .collect(),
        ),
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (convert(&key), map_object_keys(value, convert)))
                .collect::<Map<_, _>>(),
        ),
        value => value,
    }
}

fn camel_to_snake(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(input.len());

    for (index, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase() {
            let previous = index.checked_sub(1).and_then(|index| chars.get(index));
            let next = chars.get(index + 1);
            let previous_needs_separator = previous
                .map(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
                .unwrap_or(false);
            let acronym_boundary = previous.map(|ch| ch.is_ascii_uppercase()).unwrap_or(false)
                && next.map(|ch| ch.is_ascii_lowercase()).unwrap_or(false);

            if !output.is_empty() && (previous_needs_separator || acronym_boundary) {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }

    output
}

fn snake_to_camel(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut uppercase_next = false;

    for ch in input.chars() {
        if ch == '_' {
            uppercase_next = true;
            continue;
        }

        if uppercase_next {
            output.push(ch.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            output.push(ch);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn converts_params_to_rust_case_recursively() {
        assert_eq!(
            params_to_rust_case(json!({
                "leftOperand": 1,
                "URLValue": "https://example.test",
                "nestedValue": { "rightOperand": 2 },
                "items": [{ "itemValue": 3 }]
            })),
            json!({
                "left_operand": 1,
                "url_value": "https://example.test",
                "nested_value": { "right_operand": 2 },
                "items": [{ "item_value": 3 }]
            })
        );
    }

    #[test]
    fn converts_values_to_json_case_recursively() {
        assert_eq!(
            value_to_json_case(json!({
                "left_operand": 1,
                "nested_value": { "right_operand": 2 },
                "items": [{ "item_value": 3 }]
            })),
            json!({
                "leftOperand": 1,
                "nestedValue": { "rightOperand": 2 },
                "items": [{ "itemValue": 3 }]
            })
        );
    }
}
