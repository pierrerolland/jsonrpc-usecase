use serde::Serialize;
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// One failed input rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InputViolation {
    field: String,
    rule: String,
    message: String,
}

impl InputViolation {
    #[doc(hidden)]
    pub fn new(
        field: impl Into<String>,
        rule: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            rule: rule.into(),
            message: message.into(),
        }
    }

    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn rule(&self) -> &str {
        &self.rule
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn prefix_field(&mut self, prefix: &str) {
        self.field = if self.field.is_empty() {
            prefix.to_owned()
        } else {
            format!("{prefix}.{}", self.field)
        };
    }
}

/// All validation failures found on an input.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct InputValidationErrors {
    violations: Vec<InputViolation>,
}

impl InputValidationErrors {
    pub fn new() -> Self {
        Self::default()
    }

    #[doc(hidden)]
    pub fn push(&mut self, violation: InputViolation) {
        self.violations.push(violation);
    }

    #[doc(hidden)]
    pub fn extend_prefixed(&mut self, prefix: &str, mut errors: Self) {
        for violation in &mut errors.violations {
            violation.prefix_field(prefix);
        }
        self.violations.extend(errors.violations);
    }

    pub fn is_empty(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.violations.len()
    }

    pub fn violations(&self) -> &[InputViolation] {
        &self.violations
    }

    #[doc(hidden)]
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }
}

impl Display for InputValidationErrors {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "input validation failed with {} violation(s)",
            self.len()
        )
    }
}

impl Error for InputValidationErrors {}
