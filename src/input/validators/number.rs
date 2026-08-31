pub trait Number: Copy + PartialOrd {
    fn zero() -> Self;
}

macro_rules! impl_number {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Number for $type {
                fn zero() -> Self { 0 as $type }
            }
        )+
    };
}

impl_number!(u8, u16, u32, u64, u128, usize);
impl_number!(i8, i16, i32, i64, i128, isize);
impl_number!(f32, f64);
