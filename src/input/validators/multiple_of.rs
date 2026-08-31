pub trait MultipleOf: Copy {
    fn is_multiple_of_value(self, divisor: Self) -> bool;
}

macro_rules! impl_integer_multiple_of {
    ($($type:ty),+ $(,)?) => {
        $(
            impl MultipleOf for $type {
                fn is_multiple_of_value(self, divisor: Self) -> bool {
                    divisor != 0 && self % divisor == 0
                }
            }
        )+
    };
}

impl_integer_multiple_of!(u8, u16, u32, u64, u128, usize);
impl_integer_multiple_of!(i8, i16, i32, i64, i128, isize);

macro_rules! impl_float_multiple_of {
    ($($type:ty),+ $(,)?) => {
        $(
            impl MultipleOf for $type {
                fn is_multiple_of_value(self, divisor: Self) -> bool {
                    if !self.is_finite() || !divisor.is_finite() || divisor == 0.0 {
                        return false;
                    }
                    let quotient = self / divisor;
                    (quotient - quotient.round()).abs()
                        <= <$type>::EPSILON * quotient.abs().max(1.0) * 4.0
                }
            }
        )+
    };
}

impl_float_multiple_of!(f32, f64);

pub fn validate<T: MultipleOf>(value: &T, divisor: T) -> Option<String> {
    (!value.is_multiple_of_value(divisor))
        .then(|| "must be a multiple of the configured value".to_owned())
}
