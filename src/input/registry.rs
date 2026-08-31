use super::InputValidationErrors;
use std::any::{Any, TypeId};

#[doc(hidden)]
pub struct InputProcessorRegistration {
    type_id: fn() -> TypeId,
    process: fn(&mut dyn Any) -> Result<(), InputValidationErrors>,
}

impl InputProcessorRegistration {
    #[doc(hidden)]
    pub const fn new(
        type_id: fn() -> TypeId,
        process: fn(&mut dyn Any) -> Result<(), InputValidationErrors>,
    ) -> Self {
        Self { type_id, process }
    }
}

inventory::collect!(InputProcessorRegistration);

pub(crate) fn process_registered_input<T: 'static>(
    input: &mut T,
) -> Result<(), InputValidationErrors> {
    let Some(registration) = inventory::iter::<InputProcessorRegistration>
        .into_iter()
        .find(|registration| (registration.type_id)() == TypeId::of::<T>())
    else {
        return Ok(());
    };

    (registration.process)(input)
}
