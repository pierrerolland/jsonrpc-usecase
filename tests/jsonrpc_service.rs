use futures::executor::block_on;
use jsonrpc_usecase::{Error, JsonRpcService, UseCase};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt::{self, Display, Formatter};

#[derive(Default)]
struct AddNumbers;

#[derive(Deserialize)]
struct AddNumbersInput {
    left_operand: i64,
    right_operand: i64,
}

#[derive(Serialize)]
struct AddNumbersOutput {
    computed_sum: i64,
}

#[derive(Debug, Serialize)]
struct AddNumbersError {
    failure_reason: String,
}

impl Display for AddNumbersError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.failure_reason)
    }
}

impl std::error::Error for AddNumbersError {}

impl Error for AddNumbersError {
    fn code(&self) -> i64 {
        10_001
    }
}

#[UseCase]
impl AddNumbers {
    async fn execute(&self, input: AddNumbersInput) -> Result<AddNumbersOutput, AddNumbersError> {
        if input.left_operand < 0 {
            return Err(AddNumbersError {
                failure_reason: "left must be positive".to_owned(),
            });
        }

        Ok(AddNumbersOutput {
            computed_sum: input.left_operand + input.right_operand,
        })
    }
}

#[derive(Default)]
struct Ping;

#[UseCase]
impl Ping {
    async fn execute(&self, _input: ()) -> Result<&'static str, AddNumbersError> {
        Ok("pong")
    }
}

fn service() -> JsonRpcService {
    JsonRpcService::builder()
        .endpoint("/api/rpc")
        .build()
        .unwrap()
}

#[test]
fn auto_registers_macro_use_case_and_returns_result() {
    let response = block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "AddNumbers",
        "params": { "leftOperand": 2, "rightOperand": 3 },
        "id": 1
    })));

    assert_eq!(
        response,
        Some(json!({
            "jsonrpc": "2.0",
            "result": { "computedSum": 5 },
            "id": 1
        }))
    );
}

#[test]
fn converts_use_case_error_to_jsonrpc_error_object() {
    let response = block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "AddNumbers",
        "params": { "leftOperand": -1, "rightOperand": 3 },
        "id": "call-1"
    })))
    .unwrap();

    assert_eq!(response["id"], "call-1");
    assert_eq!(response["error"]["code"], 10_001);
    assert_eq!(response["error"]["message"], "AddNumbersError");
    assert_eq!(
        response["error"]["data"],
        json!({ "failureReason": "left must be positive" })
    );
}

#[test]
fn returns_standard_errors_for_jsonrpc_failures() {
    let unknown_method = block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "Missing",
        "id": 1
    })))
    .unwrap();
    assert_eq!(unknown_method["error"]["code"], -32601);

    let invalid_params = block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "AddNumbers",
        "params": { "leftOperand": 1 },
        "id": 2
    })))
    .unwrap();
    assert_eq!(invalid_params["error"]["code"], -32602);

    let invalid_request = block_on(service().handle_value(json!({
        "jsonrpc": "1.0",
        "method": "AddNumbers",
        "id": 3
    })))
    .unwrap();
    assert_eq!(invalid_request["error"]["code"], -32600);

    let parse_error = block_on(service().handle_json("{")).unwrap();
    let parse_error: Value = serde_json::from_str(&parse_error).unwrap();
    assert_eq!(parse_error["error"]["code"], -32700);
}

#[test]
fn batches_requests_and_omits_notifications() {
    let response = block_on(service().handle_value(json!([
        {
            "jsonrpc": "2.0",
            "method": "AddNumbers",
            "params": { "leftOperand": 2, "rightOperand": 3 },
            "id": "first"
        },
        {
            "jsonrpc": "2.0",
            "method": "AddNumbers",
            "params": { "leftOperand": 4, "rightOperand": 5 }
        },
        {
            "jsonrpc": "2.0",
            "method": "Ping",
            "params": [],
            "id": "second"
        }
    ])))
    .unwrap();

    assert_eq!(
        response,
        json!([
            {
                "jsonrpc": "2.0",
                "result": { "computedSum": 5 },
                "id": "first"
            },
            {
                "jsonrpc": "2.0",
                "result": "pong",
                "id": "second"
            }
        ])
    );
}

#[test]
fn notification_only_batch_returns_no_payload() {
    let response = block_on(service().handle_value(json!([
        {
            "jsonrpc": "2.0",
            "method": "AddNumbers",
            "params": { "leftOperand": 4, "rightOperand": 5 }
        }
    ])));

    assert_eq!(response, None);
}
