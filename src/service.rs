use crate::{
    config::Config,
    event::{self, EventRequest, UseCaseEvent},
    method::MethodSuccess,
    registry::UseCaseRegistration,
    request::ValidRequest,
    response::{JsonRpcErrorObject, JsonRpcResponse},
};
use futures::future::join_all;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    error::Error as StdError,
    fmt::{self, Display, Formatter},
    sync::Arc,
};

type MethodMap = HashMap<String, Arc<dyn crate::method::RpcMethod>>;

#[derive(Clone)]
pub struct JsonRpcService {
    config: Config,
    methods: Arc<MethodMap>,
}

impl JsonRpcService {
    pub fn builder() -> JsonRpcServiceBuilder {
        JsonRpcServiceBuilder::default()
    }

    pub fn new(endpoint: impl Into<String>) -> Result<Self, RegistrationError> {
        Self::builder().endpoint(endpoint).build()
    }

    pub fn endpoint(&self) -> &str {
        self.config.endpoint()
    }

    pub async fn handle_json(&self, body: &str) -> Option<String> {
        let value = match serde_json::from_str::<Value>(body) {
            Ok(value) => value,
            Err(error) => {
                let response = JsonRpcResponse::error(
                    Value::Null,
                    JsonRpcErrorObject::parse_error(Some(json!({
                        "reason": error.to_string(),
                    }))),
                );
                return Some(serialize_response(&response));
            }
        };

        let handled = self.handle_value_with_events(value).await;
        let response = handled.response.map(|value| {
            serde_json::to_string(&value).expect("JSON-RPC responses are serializable")
        });
        publish_did_events(handled.did_events);
        response
    }

    pub async fn handle_value(&self, value: Value) -> Option<Value> {
        let handled = self.handle_value_with_events(value).await;
        let response = handled.response;
        publish_did_events(handled.did_events);
        response
    }

    async fn handle_value_with_events(&self, value: Value) -> HandledValue {
        match value {
            Value::Array(items) if items.is_empty() => HandledValue {
                response: Some(
                    JsonRpcResponse::error(
                        Value::Null,
                        JsonRpcErrorObject::invalid_request(Some(json!({
                            "reason": "batch request must not be empty",
                        }))),
                    )
                    .into(),
                ),
                did_events: Vec::new(),
            },
            Value::Array(items) => {
                let handled_items =
                    join_all(items.into_iter().map(|item| self.handle_single(item))).await;
                let mut responses = Vec::new();
                let mut did_events = Vec::new();

                for handled in handled_items {
                    if let Some(response) = handled.response {
                        responses.push(Value::from(response));
                    }

                    if let Some(did_event) = handled.did_event {
                        did_events.push(did_event);
                    }
                }

                let response = if responses.is_empty() {
                    None
                } else {
                    Some(Value::Array(responses))
                };

                HandledValue {
                    response,
                    did_events,
                }
            }
            value => {
                let handled = self.handle_single(value).await;
                HandledValue {
                    response: handled.response.map(Value::from),
                    did_events: handled.did_event.into_iter().collect(),
                }
            }
        }
    }

    async fn handle_single(&self, value: Value) -> HandledSingle {
        let request = match ValidRequest::try_from(value) {
            Ok(request) => request,
            Err(error) => {
                return HandledSingle {
                    response: Some(error.into()),
                    did_event: None,
                };
            }
        };

        let id = request.id.clone();
        let result = self.execute_request(request).await;

        match (id, result) {
            (Some(id), Ok(success)) => HandledSingle {
                response: Some(JsonRpcResponse::success(id, success.output)),
                did_event: Some(success.did_event),
            },
            (Some(id), Err(error)) => HandledSingle {
                response: Some(JsonRpcResponse::error(id, error)),
                did_event: None,
            },
            (None, Ok(success)) => HandledSingle {
                response: None,
                did_event: Some(success.did_event),
            },
            (None, Err(_)) => HandledSingle {
                response: None,
                did_event: None,
            },
        }
    }

    async fn execute_request(
        &self,
        request: ValidRequest,
    ) -> Result<MethodSuccess, JsonRpcErrorObject> {
        let method_name = request.method.clone();
        let method = self.methods.get(&request.method).cloned().ok_or_else(|| {
            JsonRpcErrorObject::method_not_found(Some(json!({
                "method": method_name,
            })))
        })?;

        let event_request = EventRequest::from_valid_request(&request);
        let params = request.params;

        method.call(event_request, params).await
    }
}

struct HandledValue {
    response: Option<Value>,
    did_events: Vec<UseCaseEvent>,
}

struct HandledSingle {
    response: Option<JsonRpcResponse>,
    did_event: Option<UseCaseEvent>,
}

#[derive(Default)]
pub struct JsonRpcServiceBuilder {
    config: Config,
}

impl JsonRpcServiceBuilder {
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.config = Config::new(endpoint);
        self
    }

    pub fn build(self) -> Result<JsonRpcService, RegistrationError> {
        Ok(JsonRpcService {
            config: self.config,
            methods: Arc::new(registered_methods()?),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    DuplicateMethod(String),
    EmptyMethod,
}

impl Display for RegistrationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMethod(method) => {
                write!(
                    formatter,
                    "JSON-RPC method `{method}` is already registered"
                )
            }
            Self::EmptyMethod => formatter.write_str("JSON-RPC method name must not be empty"),
        }
    }
}

impl StdError for RegistrationError {}

fn registered_methods() -> Result<MethodMap, RegistrationError> {
    let mut methods = HashMap::new();

    for registration in inventory::iter::<UseCaseRegistration> {
        if registration.method.is_empty() {
            return Err(RegistrationError::EmptyMethod);
        }

        let method = registration.method.to_owned();
        if methods
            .insert(method.clone(), (registration.factory)())
            .is_some()
        {
            return Err(RegistrationError::DuplicateMethod(method));
        }
    }

    Ok(methods)
}

fn publish_did_events(events: Vec<UseCaseEvent>) {
    for event in events {
        event::publish_async(event);
    }
}

fn serialize_response(response: &JsonRpcResponse) -> String {
    serde_json::to_string(response).expect("JSON-RPC responses are serializable")
}
