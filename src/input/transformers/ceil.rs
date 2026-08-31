pub trait Ceil {
    fn ceil_value(self) -> Self;
}

impl Ceil for f32 {
    fn ceil_value(self) -> Self {
        self.ceil()
    }
}

impl Ceil for f64 {
    fn ceil_value(self) -> Self {
        self.ceil()
    }
}

pub fn transform<T: Ceil + Copy>(value: &mut T) {
    *value = value.ceil_value();
}
