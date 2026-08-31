pub fn transform<T>(value: &mut T, transformer: impl FnOnce(&mut T)) {
    transformer(value);
}
