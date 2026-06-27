use crate::{
    config::Config,
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

        self.handle_value(value).await.map(|value| {
            serde_json::to_string(&value).expect("JSON-RPC responses are serializable")
        })
    }

    pub async fn handle_value(&self, value: Value) -> Option<Value> {
        match value {
            Value::Array(items) if items.is_empty() => Some(
                JsonRpcResponse::error(
                    Value::Null,
                    JsonRpcErrorObject::invalid_request(Some(json!({
                        "reason": "batch request must not be empty",
                    }))),
                )
                .into(),
            ),
            Value::Array(items) => {
                let responses = join_all(items.into_iter().map(|item| self.handle_single(item)))
                    .await
                    .into_iter()
                    .flatten()
                    .map(Value::from)
                    .collect::<Vec<_>>();

                if responses.is_empty() {
                    None
                } else {
                    Some(Value::Array(responses))
                }
            }
            value => self.handle_single(value).await.map(Value::from),
        }
    }

    async fn handle_single(&self, value: Value) -> Option<JsonRpcResponse> {
        let request = match ValidRequest::try_from(value) {
            Ok(request) => request,
            Err(error) => return Some(error.into()),
        };

        let id = request.id.clone();
        let result = self.execute_request(request).await;

        id.map(|id| match result {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(error) => JsonRpcResponse::error(id, error),
        })
    }

    async fn execute_request(&self, request: ValidRequest) -> Result<Value, JsonRpcErrorObject> {
        let method_name = request.method.clone();
        let method = self.methods.get(&request.method).cloned().ok_or_else(|| {
            JsonRpcErrorObject::method_not_found(Some(json!({
                "method": method_name,
            })))
        })?;

        method.call(request.params).await
    }
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

fn serialize_response(response: &JsonRpcResponse) -> String {
    serde_json::to_string(response).expect("JSON-RPC responses are serializable")
}
