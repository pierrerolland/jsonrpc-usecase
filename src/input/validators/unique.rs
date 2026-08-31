use std::collections::HashSet;
use std::hash::Hash;

pub fn validate<T: Eq + Hash>(values: &[T]) -> Option<String> {
    let mut seen = HashSet::with_capacity(values.len());
    values
        .iter()
        .any(|value| !seen.insert(value))
        .then(|| "must not contain duplicate values".to_owned())
}
