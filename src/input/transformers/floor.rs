pub trait Floor {
    fn floor_value(self) -> Self;
}

impl Floor for f32 {
    fn floor_value(self) -> Self {
        self.floor()
    }
}

impl Floor for f64 {
    fn floor_value(self) -> Self {
        self.floor()
    }
}

pub fn transform<T: Floor + Copy>(value: &mut T) {
    *value = value.floor_value();
}
