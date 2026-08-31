use crate::input::{InputValidationErrors, UseCaseInput};

pub fn validate<T: UseCaseInput>(value: &T) -> Result<(), InputValidationErrors> {
    value.validate()
}
