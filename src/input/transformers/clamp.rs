pub fn transform<T: PartialOrd + Copy>(value: &mut T, minimum: T, maximum: T) {
    if *value < minimum {
        *value = minimum;
    } else if *value > maximum {
        *value = maximum;
    }
}
