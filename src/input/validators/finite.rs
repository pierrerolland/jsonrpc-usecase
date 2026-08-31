pub trait Finite {
    fn finite(self) -> bool;
}

impl Finite for f32 {
    fn finite(self) -> bool {
        self.is_finite()
    }
}

impl Finite for f64 {
    fn finite(self) -> bool {
        self.is_finite()
    }
}

pub fn validate<T: Finite + Copy>(value: &T) -> Option<String> {
    (!value.finite()).then(|| "must be finite".to_owned())
}
