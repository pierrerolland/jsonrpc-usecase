use crate::{JSONRPC_VERSION, response::JsonRpcErrorObject, response::JsonRpcResponse};
use serde_json::{Map, Value, json};

#[derive(Debug)]
pub(crate) struct ValidRequest {
    pub(crate) id: Option<Value>,
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

impl TryFrom<Value> for ValidRequest {
    type Error = RequestFailure;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let Value::Object(object) = value else {
            return Err(RequestFailure::invalid_request(
                Value::Null,
                "request must be a JSON object",
            ));
        };

        let response_id = response_id_from_object(&object);
        let id = request_id_from_object(&object)?;

        match object.get("jsonrpc") {
            Some(Value::String(version)) if version == JSONRPC_VERSION => {}
            _ => {
                return Err(RequestFailure::invalid_request(
                    response_id,
                    "jsonrpc must be \"2.0\"",
                ));
            }
        }

        let method = match object.get("method") {
            Some(Value::String(method)) if !method.is_empty() => method.clone(),
            _ => {
                return Err(RequestFailure::invalid_request(
                    response_id,
                    "method must be a non-empty string",
                ));
            }
        };

        Ok(Self {
            id,
            method,
            params: object.get("params").cloned(),
        })
    }
}

pub(crate) struct RequestFailure {
    id: Value,
    error: JsonRpcErrorObject,
}

impl RequestFailure {
    fn invalid_request(id: Value, reason: &str) -> Self {
        Self {
            id,
            error: JsonRpcErrorObject::invalid_request(Some(json!({
                "reason": reason,
            }))),
        }
    }
}

impl From<RequestFailure> for JsonRpcResponse {
    fn from(failure: RequestFailure) -> Self {
        JsonRpcResponse::error(failure.id, failure.error)
    }
}

fn request_id_from_object(object: &Map<String, Value>) -> Result<Option<Value>, RequestFailure> {
    if !object.contains_key("id") {
        return Ok(None);
    }

    let raw_id = object.get("id").expect("contains_key checked the id field");
    if !is_valid_id(raw_id) {
        return Err(RequestFailure::invalid_request(
            Value::Null,
            "id must be a string, number, or null",
        ));
    }

    Ok(Some(raw_id.clone()))
}

fn response_id_from_object(object: &Map<String, Value>) -> Value {
    object
        .get("id")
        .filter(|value| is_valid_id(value))
        .cloned()
        .unwrap_or(Value::Null)
}

fn is_valid_id(value: &Value) -> bool {
    matches!(value, Value::Null | Value::String(_) | Value::Number(_))
}
