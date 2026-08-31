pub trait Even {
    fn even(self) -> bool;
}

macro_rules! impl_even {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Even for $type {
                fn even(self) -> bool { self % 2 == 0 }
            }
        )+
    };
}

impl_even!(u8, u16, u32, u64, u128, usize);
impl_even!(i8, i16, i32, i64, i128, isize);

pub fn validate<T: Even + Copy>(value: &T) -> Option<String> {
    (!value.even()).then(|| "must be even".to_owned())
}
