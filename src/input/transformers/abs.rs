pub trait Absolute {
    fn absolute(self) -> Self;
}

macro_rules! impl_signed_absolute {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Absolute for $type {
                fn absolute(self) -> Self { self.saturating_abs() }
            }
        )+
    };
}

impl_signed_absolute!(i8, i16, i32, i64, i128, isize);

impl Absolute for f32 {
    fn absolute(self) -> Self {
        self.abs()
    }
}

impl Absolute for f64 {
    fn absolute(self) -> Self {
        self.abs()
    }
}

pub fn transform<T: Absolute + Copy>(value: &mut T) {
    *value = value.absolute();
}
