use crate::JSONRPC_VERSION;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub(crate) enum JsonRpcResponse {
    Success(JsonRpcSuccessResponse),
    Error(JsonRpcErrorResponse),
}

impl JsonRpcResponse {
    pub(crate) fn success(id: Value, result: Value) -> Self {
        JsonRpcSuccessResponse::new(id, result).into()
    }

    pub(crate) fn error(id: Value, error: JsonRpcErrorObject) -> Self {
        JsonRpcErrorResponse::new(id, error).into()
    }
}

impl From<JsonRpcSuccessResponse> for JsonRpcResponse {
    fn from(response: JsonRpcSuccessResponse) -> Self {
        Self::Success(response)
    }
}

impl From<JsonRpcErrorResponse> for JsonRpcResponse {
    fn from(response: JsonRpcErrorResponse) -> Self {
        Self::Error(response)
    }
}

impl From<JsonRpcResponse> for Value {
    fn from(response: JsonRpcResponse) -> Self {
        serde_json::to_value(response).expect("JSON-RPC responses are serializable")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct JsonRpcSuccessResponse {
    jsonrpc: &'static str,
    result: Value,
    id: Value,
}

impl JsonRpcSuccessResponse {
    fn new(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            result,
            id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct JsonRpcErrorResponse {
    jsonrpc: &'static str,
    error: JsonRpcErrorObject,
    id: Value,
}

impl JsonRpcErrorResponse {
    fn new(id: Value, error: JsonRpcErrorObject) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            error,
            id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JsonRpcErrorObject {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcErrorObject {
    pub(crate) const PARSE_ERROR: i64 = -32700;
    pub(crate) const INVALID_REQUEST: i64 = -32600;
    pub(crate) const METHOD_NOT_FOUND: i64 = -32601;
    pub(crate) const INVALID_PARAMS: i64 = -32602;
    pub(crate) const INTERNAL_ERROR: i64 = -32603;

    pub(crate) fn custom(code: i64, message: String, data: Option<Value>) -> Self {
        Self {
            code,
            message,
            data,
        }
    }

    pub(crate) fn parse_error(data: Option<Value>) -> Self {
        Self::standard(Self::PARSE_ERROR, "Parse error", data)
    }

    pub(crate) fn invalid_request(data: Option<Value>) -> Self {
        Self::standard(Self::INVALID_REQUEST, "Invalid Request", data)
    }

    pub(crate) fn method_not_found(data: Option<Value>) -> Self {
        Self::standard(Self::METHOD_NOT_FOUND, "Method not found", data)
    }

    pub(crate) fn invalid_params(data: Option<Value>) -> Self {
        Self::standard(Self::INVALID_PARAMS, "Invalid params", data)
    }

    pub(crate) fn internal_error(data: Option<Value>) -> Self {
        Self::standard(Self::INTERNAL_ERROR, "Internal error", data)
    }

    fn standard(code: i64, message: &str, data: Option<Value>) -> Self {
        Self {
            code,
            message: message.to_owned(),
            data,
        }
    }
}
