use unicode_normalization::UnicodeNormalization;

pub fn transform(value: &mut String) {
    *value = value.nfc().collect();
}
