use std::fmt::Display;

pub fn validate<T, E>(value: &T, validator: impl FnOnce(&T) -> Result<(), E>) -> Option<String>
where
    E: Display,
{
    validator(value).err().map(|error| error.to_string())
}
