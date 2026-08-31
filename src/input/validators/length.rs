use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub trait Length {
    fn length(&self) -> usize;
}

impl Length for str {
    fn length(&self) -> usize {
        self.chars().count()
    }
}

impl Length for String {
    fn length(&self) -> usize {
        self.as_str().length()
    }
}

impl<T> Length for [T] {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T> Length for Vec<T> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T, const N: usize> Length for [T; N] {
    fn length(&self) -> usize {
        N
    }
}

impl<K, V, S> Length for HashMap<K, V, S> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<K, V> Length for BTreeMap<K, V> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T, S> Length for HashSet<T, S> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T> Length for BTreeSet<T> {
    fn length(&self) -> usize {
        self.len()
    }
}

pub fn validate<T: Length + ?Sized>(
    value: &T,
    minimum: Option<usize>,
    maximum: Option<usize>,
    exact: Option<usize>,
) -> Option<String> {
    let length = value.length();

    if let Some(exact) = exact {
        return (length != exact).then(|| format!("must contain exactly {exact} item(s)"));
    }
    if let Some(minimum) = minimum
        && length < minimum
    {
        return Some(format!("must contain at least {minimum} item(s)"));
    }
    if let Some(maximum) = maximum
        && length > maximum
    {
        return Some(format!("must contain at most {maximum} item(s)"));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_length_counts_characters_instead_of_bytes() {
        assert_eq!(validate("é", Some(1), Some(1), None), None);
    }
}
