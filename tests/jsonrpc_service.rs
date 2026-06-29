use futures::executor::block_on;
use jsonrpc_usecase::{Error, JsonRpcService, UseCase, UseCaseEvent, UseCaseEventConsumer};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fmt::{self, Display, Formatter},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

static OBSERVED_EVENTS: Mutex<Vec<Value>> = Mutex::new(Vec::new());
static FIRST_DID_CONSUMER_CALLS: AtomicUsize = AtomicUsize::new(0);
static SECOND_DID_CONSUMER_CALLS: AtomicUsize = AtomicUsize::new(0);
static DID_CONSUMER_THREAD_IDS: Mutex<Vec<String>> = Mutex::new(Vec::new());

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

#[UseCaseEventConsumer(event = "WillAddNumbers")]
#[derive(Default)]
struct RememberWillAddNumbers;

impl RememberWillAddNumbers {
    fn consume(&self, event: &UseCaseEvent) {
        if !request_id_is(event, "event-payload-test") {
            return;
        }

        OBSERVED_EVENTS.lock().unwrap().push(json!({
            "name": event.name(),
            "request": {
                "jsonrpc": event.request().jsonrpc(),
                "method": event.request().method(),
                "params": event.request().params().cloned(),
                "id": event.request().id().cloned(),
            },
            "input": event.input().clone(),
            "output": event.output().cloned(),
        }));
    }
}

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct RememberDidAddNumbers;

impl RememberDidAddNumbers {
    fn consume(&self, event: &UseCaseEvent) {
        if request_id_is(event, "event-payload-test") {
            DID_CONSUMER_THREAD_IDS
                .lock()
                .unwrap()
                .push(current_thread_id());
            OBSERVED_EVENTS.lock().unwrap().push(json!({
                "name": event.name(),
                "request": {
                    "jsonrpc": event.request().jsonrpc(),
                    "method": event.request().method(),
                    "params": event.request().params().cloned(),
                    "id": event.request().id().cloned(),
                },
                "input": event.input().clone(),
                "output": event.output().cloned(),
            }));
        }

        if request_id_is(event, "multi-consumer-test") {
            FIRST_DID_CONSUMER_CALLS.fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct CountDidAddNumbers;

impl CountDidAddNumbers {
    fn consume(&self, event: &UseCaseEvent) {
        if request_id_is(event, "multi-consumer-test") {
            SECOND_DID_CONSUMER_CALLS.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn request_id_is(event: &UseCaseEvent, expected: &str) -> bool {
    matches!(event.request().id(), Some(Value::String(id)) if id == expected)
}

fn current_thread_id() -> String {
    format!("{:?}", thread::current().id())
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    for _ in 0..50 {
        if condition() {
            return;
        }

        thread::sleep(Duration::from_millis(10));
    }

    assert!(condition());
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
fn publishes_will_and_did_use_case_events() {
    OBSERVED_EVENTS.lock().unwrap().clear();
    DID_CONSUMER_THREAD_IDS.lock().unwrap().clear();
    let request_thread_id = current_thread_id();

    let response = block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "AddNumbers",
        "params": { "leftOperand": 2, "rightOperand": 3 },
        "id": "event-payload-test"
    })));

    assert_eq!(
        response,
        Some(json!({
            "jsonrpc": "2.0",
            "result": { "computedSum": 5 },
            "id": "event-payload-test"
        }))
    );
    wait_until(|| OBSERVED_EVENTS.lock().unwrap().len() == 2);
    assert_eq!(
        OBSERVED_EVENTS.lock().unwrap().as_slice(),
        [
            json!({
                "name": "WillAddNumbers",
                "request": {
                    "jsonrpc": "2.0",
                    "method": "AddNumbers",
                    "params": { "leftOperand": 2, "rightOperand": 3 },
                    "id": "event-payload-test",
                },
                "input": { "leftOperand": 2, "rightOperand": 3 },
                "output": null,
            }),
            json!({
                "name": "DidAddNumbers",
                "request": {
                    "jsonrpc": "2.0",
                    "method": "AddNumbers",
                    "params": { "leftOperand": 2, "rightOperand": 3 },
                    "id": "event-payload-test",
                },
                "input": { "leftOperand": 2, "rightOperand": 3 },
                "output": { "computedSum": 5 },
            }),
        ]
    );
    assert!(
        DID_CONSUMER_THREAD_IDS
            .lock()
            .unwrap()
            .iter()
            .all(|thread_id| thread_id != &request_thread_id)
    );
}

#[test]
fn supports_multiple_consumers_for_one_event() {
    FIRST_DID_CONSUMER_CALLS.store(0, Ordering::SeqCst);
    SECOND_DID_CONSUMER_CALLS.store(0, Ordering::SeqCst);

    block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "AddNumbers",
        "params": { "leftOperand": 2, "rightOperand": 3 },
        "id": "multi-consumer-test"
    })));

    wait_until(|| {
        FIRST_DID_CONSUMER_CALLS.load(Ordering::SeqCst) == 1
            && SECOND_DID_CONSUMER_CALLS.load(Ordering::SeqCst) == 1
    });
    assert_eq!(FIRST_DID_CONSUMER_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(SECOND_DID_CONSUMER_CALLS.load(Ordering::SeqCst), 1);
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
