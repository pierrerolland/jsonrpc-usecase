use crate::{
    case,
    event::{self, UseCaseEvent},
    guard::GuardContext,
    response::JsonRpcErrorObject,
    use_case::UseCaseDefinition,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{future::Future, pin::Pin};

pub trait RpcMethod: Send + Sync {
    fn call<'a>(
        &'a self,
        context: GuardContext,
        params: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<MethodSuccess, JsonRpcErrorObject>> + Send + 'a>>;
}

pub struct MethodSuccess {
    pub(crate) output: Value,
    pub(crate) did_event: UseCaseEvent,
}

pub struct UseCaseMethod<U> {
    use_case: U,
}

impl<U> From<U> for UseCaseMethod<U> {
    fn from(use_case: U) -> Self {
        Self { use_case }
    }
}

impl<U> RpcMethod for UseCaseMethod<U>
where
    U: UseCaseDefinition,
{
    fn call<'a>(
        &'a self,
        context: GuardContext,
        params: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<MethodSuccess, JsonRpcErrorObject>> + Send + 'a>> {
        Box::pin(async move {
            if !U::can_proceed(&context) {
                return Err(JsonRpcErrorObject::access_denied(Some(json!({
                    "method": context.request().method(),
                }))));
            }

            let event_request = context.request().clone();
            let params = params.unwrap_or(Value::Null);
            let (input, event_input) = deserialize_input::<U::Input>(params)?;

            event::publish(&UseCaseEvent::will(
                U::WILL_EVENT,
                event_request.clone(),
                event_input.clone(),
            ));

            let output = self
                .use_case
                .execute(input)
                .await
                .map_err(|error| JsonRpcErrorObject::from(&error))?;

            let output = serde_json::to_value(&output)
                .map(case::value_to_json_case)
                .map_err(|error| {
                    JsonRpcErrorObject::internal_error(Some(json!({
                        "reason": error.to_string(),
                    })))
                })?;

            let did_event =
                UseCaseEvent::did(U::DID_EVENT, event_request, event_input, output.clone());

            Ok(MethodSuccess { output, did_event })
        })
    }
}

fn deserialize_input<T>(params: Value) -> Result<(T, Value), JsonRpcErrorObject>
where
    T: DeserializeOwned,
{
    let params = case::params_to_rust_case(params);

    match serde_json::from_value::<T>(params.clone()) {
        Ok(input) => Ok((input, case::value_to_json_case(params))),
        Err(error) if matches!(params, Value::Array(ref items) if items.is_empty()) => {
            serde_json::from_value::<T>(Value::Null)
                .map(|input| (input, Value::Null))
                .map_err(|_| invalid_params(error))
        }
        Err(error) => Err(invalid_params(error)),
    }
}

fn invalid_params(error: serde_json::Error) -> JsonRpcErrorObject {
    JsonRpcErrorObject::invalid_params(Some(json!({
        "reason": error.to_string(),
    })))
}
