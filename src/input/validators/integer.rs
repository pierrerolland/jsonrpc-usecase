pub trait IntegerValue {
    fn integer_value(self) -> bool;
}

macro_rules! impl_integer_value {
    ($($type:ty),+ $(,)?) => {
        $(
            impl IntegerValue for $type {
                fn integer_value(self) -> bool { true }
            }
        )+
    };
}

impl_integer_value!(u8, u16, u32, u64, u128, usize);
impl_integer_value!(i8, i16, i32, i64, i128, isize);

impl IntegerValue for f32 {
    fn integer_value(self) -> bool {
        self.is_finite() && self.fract() == 0.0
    }
}

impl IntegerValue for f64 {
    fn integer_value(self) -> bool {
        self.is_finite() && self.fract() == 0.0
    }
}

pub fn validate<T: IntegerValue + Copy>(value: &T) -> Option<String> {
    (!value.integer_value()).then(|| "must be an integer".to_owned())
}
