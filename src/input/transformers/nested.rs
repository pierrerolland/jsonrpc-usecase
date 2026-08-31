use crate::input::UseCaseInput;

pub fn transform<T: UseCaseInput>(value: &mut T) {
    value.transform();
}
