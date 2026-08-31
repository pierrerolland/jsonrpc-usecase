use crate::{
    case,
    context::RequestContext,
    event::{self, UseCaseEvent},
    guard::GuardContext,
    input::InputValidationErrors,
    response::JsonRpcErrorObject,
    use_case::UseCaseDefinition,
};
use serde_json::{Value, json};
use std::{future::Future, pin::Pin, sync::Arc};

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
            let request_context: RequestContext = context.context().clone();
            let params = params.unwrap_or(Value::Null);
            let DeserializedInput {
                input,
                event_input,
                will_event_input,
                did_event_input,
            } = deserialize_input::<U>(params)?;

            event::publish(&UseCaseEvent::will_typed(
                U::WILL_EVENT,
                event_request.clone(),
                request_context.clone(),
                event_input.clone(),
                Arc::clone(&will_event_input),
            ))
            .await;

            let typed_output = self
                .use_case
                .execute_prepared(input)
                .await
                .map_err(|error| JsonRpcErrorObject::from(&error))?;

            let output = serde_json::to_value(&typed_output)
                .map(case::value_to_json_case)
                .map_err(|error| {
                    JsonRpcErrorObject::internal_error(Some(json!({
                        "reason": error.to_string(),
                    })))
                })?;

            let did_event = UseCaseEvent::did_typed(
                U::DID_EVENT,
                event_request,
                request_context,
                event_input,
                output.clone(),
                did_event_input,
                typed_output,
            );

            Ok(MethodSuccess { output, did_event })
        })
    }
}

struct DeserializedInput<T> {
    input: T,
    event_input: Value,
    will_event_input: Arc<T>,
    did_event_input: Arc<T>,
}

type InputCopies<T> = (T, Arc<T>, Arc<T>);

fn deserialize_input<U>(params: Value) -> Result<DeserializedInput<U::Input>, JsonRpcErrorObject>
where
    U: UseCaseDefinition,
{
    let params = case::params_to_rust_case(params);

    match deserialize_input_values::<U>(&params) {
        Ok((input, will_event_input, did_event_input)) => Ok(DeserializedInput {
            input,
            event_input: case::value_to_json_case(params),
            will_event_input,
            did_event_input,
        }),
        Err(InputFailure::Deserialize(error)) if matches!(params, Value::Array(ref items) if items.is_empty()) => {
            deserialize_input_values::<U>(&Value::Null)
                .map(
                    |(input, will_event_input, did_event_input)| DeserializedInput {
                        input,
                        event_input: Value::Null,
                        will_event_input,
                        did_event_input,
                    },
                )
                .map_err(|_| invalid_params(InputFailure::Deserialize(error)))
        }
        Err(error) => Err(invalid_params(error)),
    }
}

fn deserialize_input_values<U>(params: &Value) -> Result<InputCopies<U::Input>, InputFailure>
where
    U: UseCaseDefinition,
{
    let input = U::prepare_input(serde_json::from_value::<U::Input>(params.clone())?)?;
    let will_event_input = U::prepare_input(serde_json::from_value::<U::Input>(params.clone())?)?;
    let did_event_input = U::prepare_input(serde_json::from_value::<U::Input>(params.clone())?)?;

    let event_input = Arc::new(will_event_input);

    Ok((input, Arc::clone(&event_input), Arc::new(did_event_input)))
}

enum InputFailure {
    Deserialize(serde_json::Error),
    Validation(InputValidationErrors),
}

impl From<serde_json::Error> for InputFailure {
    fn from(error: serde_json::Error) -> Self {
        Self::Deserialize(error)
    }
}

impl From<InputValidationErrors> for InputFailure {
    fn from(errors: InputValidationErrors) -> Self {
        Self::Validation(errors)
    }
}

fn invalid_params(error: InputFailure) -> JsonRpcErrorObject {
    let data = match error {
        InputFailure::Deserialize(error) => json!({ "reason": error.to_string() }),
        InputFailure::Validation(errors) => json!({ "violations": errors.violations() }),
    };

    JsonRpcErrorObject::invalid_params(Some(data))
}
