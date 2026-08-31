pub trait Round {
    fn round_to(self, precision: i32) -> Self;
}

impl Round for f32 {
    fn round_to(self, precision: i32) -> Self {
        let factor = 10_f32.powi(precision);
        (self * factor).round() / factor
    }
}

impl Round for f64 {
    fn round_to(self, precision: i32) -> Self {
        let factor = 10_f64.powi(precision);
        (self * factor).round() / factor
    }
}

pub fn transform<T: Round + Copy>(value: &mut T, precision: i32) {
    *value = value.round_to(precision);
}
