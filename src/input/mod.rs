mod error;
mod registry;

#[doc(hidden)]
pub mod transformers;
#[doc(hidden)]
pub mod validators;

pub use error::{InputValidationErrors, InputViolation};
pub use registry::InputProcessorRegistration;
pub(crate) use registry::process_registered_input;

/// Transforms and validates a use-case input independently of its transport.
///
/// Derive this trait with [`jsonrpc_usecase::UseCaseInput`]. JSON-RPC dispatch
/// discovers derived inputs automatically, while other callers can invoke
/// [`UseCaseInput::process`] and propagate [`InputValidationErrors`] directly.
pub trait UseCaseInput: 'static {
    /// Applies all configured transformations in declaration order.
    fn transform(&mut self);

    /// Runs every configured validator and returns all violations.
    fn validate(&self) -> Result<(), InputValidationErrors>;

    /// Applies transformations and then validates the resulting value.
    fn process(&mut self) -> Result<(), InputValidationErrors> {
        self.transform();
        self.validate()
    }

    /// Processes an owned input and returns the transformed value.
    ///
    /// This is convenient when one use case invokes another: the caller can use
    /// `?` to propagate the transport-independent validation error.
    fn into_processed(mut self) -> Result<Self, InputValidationErrors>
    where
        Self: Sized,
    {
        self.process()?;
        Ok(self)
    }
}
