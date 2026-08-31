use crate::InputValidationErrors;
use std::{
    error::Error as StdError,
    fmt::{self, Display, Formatter},
};

/// A failure produced while executing a use case through its validated boundary.
#[derive(Debug)]
pub enum UseCaseExecutionError<E> {
    /// The input failed one or more use-case input rules.
    InvalidInput(InputValidationErrors),
    /// The validated input reached the use case, which returned its own error.
    Execution(E),
}

impl<E> UseCaseExecutionError<E> {
    /// Returns the violations when input validation failed.
    pub fn input_errors(&self) -> Option<&InputValidationErrors> {
        match self {
            Self::InvalidInput(errors) => Some(errors),
            Self::Execution(_) => None,
        }
    }

    /// Returns the use case's own error when its prepared implementation failed.
    pub fn execution_error(&self) -> Option<&E> {
        match self {
            Self::InvalidInput(_) => None,
            Self::Execution(error) => Some(error),
        }
    }
}

impl<E> From<InputValidationErrors> for UseCaseExecutionError<E> {
    fn from(errors: InputValidationErrors) -> Self {
        Self::InvalidInput(errors)
    }
}

impl<E: Display> Display for UseCaseExecutionError<E> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(errors) => Display::fmt(errors, formatter),
            Self::Execution(error) => Display::fmt(error, formatter),
        }
    }
}

impl<E: StdError + 'static> StdError for UseCaseExecutionError<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::InvalidInput(errors) => Some(errors),
            Self::Execution(error) => Some(error),
        }
    }
}
