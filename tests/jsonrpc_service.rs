use futures::executor::block_on;
use jsonrpc_usecase::{
    Error, Guard, GuardContext, JsonRpcService, RequestHeaders, UseCase, UseCaseEvent,
    UseCaseEventConsumer, current_context,
};
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
static TYPED_DID_OUTPUTS: Mutex<Vec<i64>> = Mutex::new(Vec::new());
static OBSERVED_CONTEXT_EVENTS: Mutex<Vec<Value>> = Mutex::new(Vec::new());
static CONTEXT_BUILDER_CALLS: AtomicUsize = AtomicUsize::new(0);

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

#[derive(Default)]
struct RequireAccessHeader;

impl Guard for RequireAccessHeader {
    fn can_proceed(&self, context: &GuardContext) -> bool {
        context.request().method() == "GuardedEcho"
            && context.headers().get("x-access-token") == Some("allowed")
    }
}

#[derive(Default)]
struct GuardedEcho;

#[derive(Deserialize)]
struct GuardedEchoInput {
    value: String,
}

#[derive(Serialize)]
struct GuardedEchoOutput {
    value: String,
}

#[UseCase(jsonrpc = true, guards = [RequireAccessHeader])]
impl GuardedEcho {
    async fn execute(&self, input: GuardedEchoInput) -> Result<GuardedEchoOutput, AddNumbersError> {
        Ok(GuardedEchoOutput { value: input.value })
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

#[derive(Clone, Debug)]
struct CallerContext {
    user_id: String,
    role: String,
    trace_id: String,
}

#[derive(Default)]
struct DescribeCaller;

#[derive(Serialize)]
struct DescribeCallerOutput {
    user_id: String,
    role: String,
    trace_id: String,
}

#[UseCase]
impl DescribeCaller {
    async fn execute(&self, _input: ()) -> Result<DescribeCallerOutput, AddNumbersError> {
        let context = current_context::<CallerContext>().expect("caller context should be present");

        Ok(DescribeCallerOutput {
            user_id: context.user_id.clone(),
            role: context.role.clone(),
            trace_id: context.trace_id.clone(),
        })
    }
}

#[derive(Default)]
struct RequireAdminContext;

impl Guard for RequireAdminContext {
    fn can_proceed(&self, context: &GuardContext) -> bool {
        context
            .get_context::<CallerContext>()
            .is_some_and(|caller| caller.role == "admin")
    }
}

#[derive(Default)]
struct AdminEcho;

#[UseCase(guards = [RequireAdminContext])]
impl AdminEcho {
    async fn execute(&self, input: GuardedEchoInput) -> Result<GuardedEchoOutput, AddNumbersError> {
        Ok(GuardedEchoOutput { value: input.value })
    }
}

#[UseCaseEventConsumer(event = "WillAddNumbers")]
#[derive(Default)]
struct RememberWillAddNumbers;

impl RememberWillAddNumbers {
    async fn consume(&self, event: &UseCaseEvent) {
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
            "typedInput": {
                "leftOperand": event.get_input::<AddNumbersInput>().unwrap().left_operand,
                "rightOperand": event.get_input::<AddNumbersInput>().unwrap().right_operand,
            },
            "output": event.output().cloned(),
        }));
    }
}

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct RememberDidAddNumbers;

impl RememberDidAddNumbers {
    async fn consume(&self, event: &UseCaseEvent) {
        tokio::time::sleep(Duration::from_millis(1)).await;

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
                "typedInput": {
                    "leftOperand": event.get_input::<AddNumbersInput>().unwrap().left_operand,
                    "rightOperand": event.get_input::<AddNumbersInput>().unwrap().right_operand,
                },
                "output": event.output().cloned(),
                "typedOutput": event.get_output::<AddNumbersOutput>().unwrap().computed_sum,
            }));
        }

        if request_id_is(event, "multi-consumer-test") {
            FIRST_DID_CONSUMER_CALLS.fetch_add(1, Ordering::SeqCst);
        }

        if request_id_is(event, "typed-output-test") {
            TYPED_DID_OUTPUTS
                .lock()
                .unwrap()
                .push(event.get_output::<AddNumbersOutput>().unwrap().computed_sum);
        }
    }
}

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct CountDidAddNumbers;

impl CountDidAddNumbers {
    async fn consume(&self, event: &UseCaseEvent) {
        if request_id_is(event, "multi-consumer-test") {
            SECOND_DID_CONSUMER_CALLS.fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[UseCaseEventConsumer(event = "DidDescribeCaller")]
#[derive(Default)]
struct RememberDidDescribeCaller;

impl RememberDidDescribeCaller {
    async fn consume(&self, event: &UseCaseEvent) {
        if !request_id_is(event, "context-event-test") {
            return;
        }

        let context = event
            .get_context::<CallerContext>()
            .expect("event should carry caller context");
        let scoped_context =
            current_context::<CallerContext>().expect("event consumer should be context-scoped");

        OBSERVED_CONTEXT_EVENTS.lock().unwrap().push(json!({
            "userId": context.user_id.as_str(),
            "role": context.role.as_str(),
            "traceId": context.trace_id.as_str(),
            "scopedUserId": scoped_context.user_id.as_str(),
        }));
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

struct InternalPing;

#[UseCase(method = "Ping", jsonrpc = false)]
impl InternalPing {
    async fn execute(&self, _input: ()) -> Result<&'static str, AddNumbersError> {
        Ok("internal pong")
    }
}

fn service() -> JsonRpcService {
    JsonRpcService::builder()
        .endpoint("/api/rpc")
        .build()
        .unwrap()
}

fn context_service() -> JsonRpcService {
    JsonRpcService::builder()
        .endpoint("/api/rpc")
        .context_builder(|request| CallerContext {
            user_id: request
                .headers()
                .get("x-user-id")
                .unwrap_or("anonymous")
                .to_owned(),
            role: request
                .headers()
                .get("x-role")
                .unwrap_or("guest")
                .to_owned(),
            trace_id: request
                .headers()
                .get("x-trace-id")
                .unwrap_or("missing")
                .to_owned(),
        })
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
fn disabled_registration_does_not_conflict_with_an_existing_method() {
    let response = block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "Ping",
        "id": 1,
    })));

    assert_eq!(
        response,
        Some(json!({
            "jsonrpc": "2.0",
            "result": "pong",
            "id": 1,
        }))
    );
    assert_eq!(block_on(InternalPing.execute(())).unwrap(), "internal pong");
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
                "typedInput": { "leftOperand": 2, "rightOperand": 3 },
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
                "typedInput": { "leftOperand": 2, "rightOperand": 3 },
                "output": { "computedSum": 5 },
                "typedOutput": 5,
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
fn event_consumers_can_read_typed_output() {
    TYPED_DID_OUTPUTS.lock().unwrap().clear();

    block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "AddNumbers",
        "params": { "leftOperand": 8, "rightOperand": 13 },
        "id": "typed-output-test"
    })));

    wait_until(|| TYPED_DID_OUTPUTS.lock().unwrap().as_slice() == [21]);
    assert_eq!(TYPED_DID_OUTPUTS.lock().unwrap().as_slice(), [21]);
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
fn denies_guarded_use_case_when_guard_rejects_request() {
    let response = block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "GuardedEcho",
        "params": { "value": "secret" },
        "id": "guarded"
    })))
    .unwrap();

    assert_eq!(response["id"], "guarded");
    assert_eq!(response["error"]["code"], -32001);
    assert_eq!(response["error"]["message"], "Access denied");
    assert_eq!(
        response["error"]["data"],
        json!({ "method": "GuardedEcho" })
    );
}

#[test]
fn allows_guarded_use_case_when_guard_accepts_headers_and_request() {
    let response = block_on(service().handle_value_with_headers(
        json!({
            "jsonrpc": "2.0",
            "method": "GuardedEcho",
            "params": { "value": "secret" },
            "id": "guarded"
        }),
        RequestHeaders::new([("X-Access-Token", "allowed")]),
    ));

    assert_eq!(
        response,
        Some(json!({
            "jsonrpc": "2.0",
            "result": { "value": "secret" },
            "id": "guarded"
        }))
    );
}

#[test]
fn use_cases_can_read_context_built_from_headers() {
    let response = block_on(context_service().handle_value_with_headers(
        json!({
            "jsonrpc": "2.0",
            "method": "DescribeCaller",
            "params": [],
            "id": "caller"
        }),
        RequestHeaders::new([
            ("X-User-Id", "user-123"),
            ("X-Role", "member"),
            ("X-Trace-Id", "trace-abc"),
        ]),
    ));

    assert_eq!(
        response,
        Some(json!({
            "jsonrpc": "2.0",
            "result": {
                "userId": "user-123",
                "role": "member",
                "traceId": "trace-abc",
            },
            "id": "caller"
        }))
    );
}

#[test]
fn guards_can_read_context_built_from_headers() {
    let denied = block_on(context_service().handle_value_with_headers(
        json!({
            "jsonrpc": "2.0",
            "method": "AdminEcho",
            "params": { "value": "secret" },
            "id": "admin-denied"
        }),
        RequestHeaders::new([("X-Role", "member")]),
    ))
    .unwrap();

    assert_eq!(denied["error"]["code"], -32001);
    assert_eq!(denied["error"]["message"], "Access denied");

    let allowed = block_on(context_service().handle_value_with_headers(
        json!({
            "jsonrpc": "2.0",
            "method": "AdminEcho",
            "params": { "value": "secret" },
            "id": "admin-allowed"
        }),
        RequestHeaders::new([("X-Role", "admin")]),
    ));

    assert_eq!(
        allowed,
        Some(json!({
            "jsonrpc": "2.0",
            "result": { "value": "secret" },
            "id": "admin-allowed"
        }))
    );
}

#[test]
fn event_consumers_can_read_context() {
    OBSERVED_CONTEXT_EVENTS.lock().unwrap().clear();

    block_on(context_service().handle_value_with_headers(
        json!({
            "jsonrpc": "2.0",
            "method": "DescribeCaller",
            "params": [],
            "id": "context-event-test"
        }),
        RequestHeaders::new([
            ("X-User-Id", "event-user"),
            ("X-Role", "auditor"),
            ("X-Trace-Id", "event-trace"),
        ]),
    ));

    wait_until(|| OBSERVED_CONTEXT_EVENTS.lock().unwrap().len() == 1);
    assert_eq!(
        OBSERVED_CONTEXT_EVENTS.lock().unwrap().as_slice(),
        [json!({
            "userId": "event-user",
            "role": "auditor",
            "traceId": "event-trace",
            "scopedUserId": "event-user",
        })]
    );
}

#[test]
fn context_builder_runs_once_per_http_request() {
    CONTEXT_BUILDER_CALLS.store(0, Ordering::SeqCst);

    let service = JsonRpcService::builder()
        .endpoint("/api/rpc")
        .context_builder(|request| {
            CONTEXT_BUILDER_CALLS.fetch_add(1, Ordering::SeqCst);

            CallerContext {
                user_id: request
                    .headers()
                    .get("x-user-id")
                    .unwrap_or("anonymous")
                    .to_owned(),
                role: request
                    .headers()
                    .get("x-role")
                    .unwrap_or("guest")
                    .to_owned(),
                trace_id: request
                    .headers()
                    .get("x-trace-id")
                    .unwrap_or("missing")
                    .to_owned(),
            }
        })
        .build()
        .unwrap();

    let response = block_on(service.handle_value_with_headers(
        json!([
            {
                "jsonrpc": "2.0",
                "method": "DescribeCaller",
                "params": [],
                "id": "first-context"
            },
            {
                "jsonrpc": "2.0",
                "method": "DescribeCaller",
                "params": [],
                "id": "second-context"
            }
        ]),
        RequestHeaders::new([
            ("X-User-Id", "batch-user"),
            ("X-Role", "member"),
            ("X-Trace-Id", "batch-trace"),
        ]),
    ));

    assert_eq!(CONTEXT_BUILDER_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(
        response,
        Some(json!([
            {
                "jsonrpc": "2.0",
                "result": {
                    "userId": "batch-user",
                    "role": "member",
                    "traceId": "batch-trace",
                },
                "id": "first-context"
            },
            {
                "jsonrpc": "2.0",
                "result": {
                    "userId": "batch-user",
                    "role": "member",
                    "traceId": "batch-trace",
                },
                "id": "second-context"
            }
        ]))
    );
}

#[test]
fn async_context_builder_can_build_context() {
    let service = JsonRpcService::builder()
        .endpoint("/api/rpc")
        .async_context_builder(|request| async move {
            CallerContext {
                user_id: request
                    .headers()
                    .get("x-user-id")
                    .unwrap_or("anonymous")
                    .to_owned(),
                role: request
                    .headers()
                    .get("x-role")
                    .unwrap_or("guest")
                    .to_owned(),
                trace_id: request
                    .headers()
                    .get("x-trace-id")
                    .unwrap_or("missing")
                    .to_owned(),
            }
        })
        .build()
        .unwrap();

    let response = block_on(service.handle_value_with_headers(
        json!({
            "jsonrpc": "2.0",
            "method": "DescribeCaller",
            "params": [],
            "id": "async-context"
        }),
        RequestHeaders::new([
            ("X-User-Id", "async-user"),
            ("X-Role", "member"),
            ("X-Trace-Id", "async-trace"),
        ]),
    ));

    assert_eq!(
        response,
        Some(json!({
            "jsonrpc": "2.0",
            "result": {
                "userId": "async-user",
                "role": "member",
                "traceId": "async-trace",
            },
            "id": "async-context"
        }))
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
