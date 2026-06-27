use crate::{case, response::JsonRpcErrorObject};
use serde::Serialize;
use serde_json::{Value, json};
use std::{any::type_name, borrow::Cow, error::Error as StdError};

pub trait Error: StdError + Serialize + Send + Sync + 'static {
    fn code(&self) -> i64;

    fn message(&self) -> Cow<'static, str> {
        Cow::Borrowed(short_type_name::<Self>())
    }

    fn data(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|error| {
            json!({
                "serializationError": error.to_string(),
            })
        })
    }
}

impl<E> From<&E> for JsonRpcErrorObject
where
    E: Error + ?Sized,
{
    fn from(error: &E) -> Self {
        Self::custom(
            error.code(),
            error.message().into_owned(),
            Some(case::value_to_json_case(error.data())),
        )
    }
}

fn short_type_name<T: ?Sized>() -> &'static str {
    type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or(type_name::<T>())
}
