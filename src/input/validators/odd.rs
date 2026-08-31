pub trait Odd {
    fn odd(self) -> bool;
}

macro_rules! impl_odd {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Odd for $type {
                fn odd(self) -> bool { self % 2 != 0 }
            }
        )+
    };
}

impl_odd!(u8, u16, u32, u64, u128, usize);
impl_odd!(i8, i16, i32, i64, i128, isize);

pub fn validate<T: Odd + Copy>(value: &T) -> Option<String> {
    (!value.odd()).then(|| "must be odd".to_owned())
}
