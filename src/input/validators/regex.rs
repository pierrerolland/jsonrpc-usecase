use regex::Regex;
use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

static CACHE: OnceLock<RwLock<HashMap<&'static str, Option<Regex>>>> = OnceLock::new();

pub fn validate(value: &str, pattern: &'static str) -> Option<String> {
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    {
        let cache = cache
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(expression) = cache.get(pattern) {
            return validation_message(value, pattern, expression.as_ref());
        }
    }

    let mut cache = cache
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let expression = cache
        .entry(pattern)
        .or_insert_with(|| Regex::new(pattern).ok());
    validation_message(value, pattern, expression.as_ref())
}

fn validation_message(value: &str, pattern: &str, expression: Option<&Regex>) -> Option<String> {
    match expression {
        Some(expression) if expression.is_match(value) => None,
        Some(_) => Some(format!("must match regular expression {pattern:?}")),
        None => Some(format!("uses invalid regular expression {pattern:?}")),
    }
}
