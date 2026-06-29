# jsonrpc-usecase

`jsonrpc-usecase` turns Rust use cases into JSON-RPC 2.0 methods with a single macro.

The intended workflow is:

1. Define a struct for the use case.
2. Implement an inherent `async fn execute(&self, input) -> Result<output, error>` method.
3. Put `#[UseCase]` on that `impl` block.
4. Build a `JsonRpcService`; all macro-marked use cases are discovered automatically.

No manual use-case registration is required.

## Public API

Most application code only needs:

```rust
use jsonrpc_usecase::{Error, JsonRpcService, UseCase};
```

Or:

```rust
use jsonrpc_usecase::prelude::*;
```

Developer-facing items:

- `UseCase`: attribute macro applied to an inherent `impl` block.
- `UseCaseEventConsumer`: attribute macro applied to an event consumer struct, function, or impl block.
- `Error`: trait implemented by application error types.
- `Guard`: trait implemented by access-control guard types.
- `JsonRpcService`: framework-neutral JSON-RPC handler.
- `JsonRpcServiceBuilder`: builder returned by `JsonRpcService::builder()`.
- `EventRequest` and `UseCaseEvent`: event payload types.
- `GuardContext` and `RequestHeaders`: guard payload types.
- `RegistrationError`: returned when the auto-registration registry is invalid, for example duplicate method names.

The JSON-RPC request parser, response DTOs, dispatcher, registry, and macro support module are internal. Treat responses as JSON returned by `JsonRpcService`.

## Install

```toml
[dependencies]
jsonrpc-usecase = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

When using this repository locally:

```toml
[dependencies]
jsonrpc-usecase = { path = "../jsonrpc-usecase" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

For the optional Axum adapter:

```toml
[dependencies]
jsonrpc-usecase = { version = "0.3", features = ["axum"] }
```

## Define A Use Case

```rust,ignore
use jsonrpc_usecase::{Error, UseCase};
use serde::{Deserialize, Serialize};
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
        if input.left_operand < 0 || input.right_operand < 0 {
            return Err(AddNumbersError {
                failure_reason: "only positive numbers are accepted".to_owned(),
            });
        }

        Ok(AddNumbersOutput {
            computed_sum: input.left_operand + input.right_operand,
        })
    }
}
```

The macro validates that the `impl` block contains this shape:

```rust,ignore
async fn execute(&self, input: Input) -> Result<Output, Error>
```

The macro also implements the hidden runtime trait and submits the use case to the global registry.

Current constraint: macro-registered use-case structs must implement `Default`, because the service can no longer receive explicit instances during registration.

## Method Names

Method names are PascalCase by default. The struct name is used as the JSON-RPC method name.

```rust,ignore
#[derive(Default)]
struct AddNumbers;

#[UseCase]
impl AddNumbers {
    async fn execute(&self, input: AddNumbersInput) -> Result<AddNumbersOutput, AddNumbersError> {
        todo!()
    }
}
```

This registers the method `AddNumbers`.

You can override the method name when needed:

```rust,ignore
#[UseCase(method = "MathAdd")]
impl AddNumbers {
    async fn execute(&self, input: AddNumbersInput) -> Result<AddNumbersOutput, AddNumbersError> {
        todo!()
    }
}
```

## Guards

Use guards to deny access before a use case deserializes input or executes. A guard implements `Guard` and receives a `GuardContext` containing HTTP request headers and the validated JSON-RPC request snapshot.

```rust,ignore
use jsonrpc_usecase::{Guard, GuardContext, UseCase};

#[derive(Default)]
struct RequireAccessToken;

impl Guard for RequireAccessToken {
    fn can_proceed(&self, context: &GuardContext) -> bool {
        context.headers().get("x-access-token") == Some("allowed")
            && context.request().method() == "ReadSecret"
    }
}

#[derive(Default)]
struct ReadSecret;

#[UseCase(guards = [RequireAccessToken])]
impl ReadSecret {
    async fn execute(&self, input: ReadSecretInput) -> Result<ReadSecretOutput, ReadSecretError> {
        todo!()
    }
}
```

You can combine guards. All guards must return `true`:

```rust,ignore
#[UseCase(guards = [RequireAccessToken, RequireAdminRole])]
impl ReadSecret {
    async fn execute(&self, input: ReadSecretInput) -> Result<ReadSecretOutput, ReadSecretError> {
        todo!()
    }
}
```

Guards are instantiated with `Default`, so guard types must implement `Default`. Header lookup is case-insensitive. If any guard returns `false`, the use case is not called and the JSON-RPC response uses code `-32001` with message `Access denied`.

Framework-neutral callers can pass headers explicitly:

```rust,ignore
use jsonrpc_usecase::{JsonRpcService, RequestHeaders};
use serde_json::json;

let response = service.handle_value_with_headers(
    json!({
        "jsonrpc": "2.0",
        "method": "ReadSecret",
        "id": 1
    }),
    RequestHeaders::new([("X-Access-Token", "allowed")]),
).await;
```

`handle_json` and `handle_value` use an empty header set. The Axum adapter passes incoming HTTP request headers automatically.

## Build The Service

All `#[UseCase]` impl blocks linked into the binary are auto-registered when the service is built.

```rust
use jsonrpc_usecase::{JsonRpcService, RegistrationError};

fn build_service() -> Result<JsonRpcService, RegistrationError> {
    JsonRpcService::builder()
        .endpoint("/rpc")
        .build()
}
```

There is no `register(...)` call. If two use cases register the same JSON-RPC method name, `build()` returns `RegistrationError::DuplicateMethod`.

## Handle Requests

Use `handle_json` when your transport gives you a raw JSON body:

```rust,ignore
let request = r#"{
    "jsonrpc": "2.0",
    "method": "AddNumbers",
    "params": { "leftOperand": 2, "rightOperand": 3 },
    "id": 1
}"#;

let response = service.handle_json(request).await;

assert_eq!(
    response.as_deref(),
    Some(r#"{"jsonrpc":"2.0","result":{"computedSum":5},"id":1}"#)
);
```

Use `handle_value` when your framework already parsed JSON into `serde_json::Value`:

```rust,ignore
use serde_json::json;

let response = service.handle_value(json!({
    "jsonrpc": "2.0",
    "method": "AddNumbers",
    "params": { "leftOperand": 2, "rightOperand": 3 },
    "id": "request-1"
})).await;

assert_eq!(response, Some(json!({
    "jsonrpc": "2.0",
    "result": { "computedSum": 5 },
    "id": "request-1"
})));
```

Both handlers return `Option` because JSON-RPC notifications do not produce a response. A request without an `id` is executed, but returns `None`.

## Use-Case Events

Every `#[UseCase]` impl publishes two named events around successful use-case execution:

- `Will<UseCaseName>` after the JSON-RPC params have been validated and deserialized, immediately before `execute`.
- `Did<UseCaseName>` after `execute` returns `Ok(output)` and the output has been serialized.

For `AddNumbers`, the event names are `WillAddNumbers` and `DidAddNumbers`. Event names are based on the use-case struct name, even when the JSON-RPC method name is overridden with `#[UseCase(method = "...")]`.

Register consumers as standalone structs:

```rust,ignore
use jsonrpc_usecase::{UseCaseEvent, UseCaseEventConsumer};

#[UseCaseEventConsumer(event = "WillAddNumbers")]
#[derive(Default)]
struct AuditAddNumbersRequest;

impl AuditAddNumbersRequest {
    fn consume(&self, event: &UseCaseEvent) {
        let method = event.request().method();
        let input = event.input();
    }
}

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct AuditAddNumbersResult;

impl AuditAddNumbersResult {
    fn consume(&self, event: &UseCaseEvent) {
        let request_id = event.request().id();
        let output = event.output();
    }
}
```

All linked event consumers are auto-discovered. There is no service builder registration step. Struct consumers must implement `Default` and have a `consume(&self, event: &UseCaseEvent)` method.

Multiple consumers can listen to the same event by using the same event name more than once:

```rust,ignore
use jsonrpc_usecase::{UseCaseEvent, UseCaseEventConsumer};

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct WriteAddNumbersAuditLog;

impl WriteAddNumbersAuditLog {
    fn consume(&self, event: &UseCaseEvent) {
        println!("audit event: {}", event.name());
    }
}

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct UpdateAddNumbersMetrics;

impl UpdateAddNumbersMetrics {
    fn consume(&self, event: &UseCaseEvent) {
        println!("metric event: {}", event.name());
    }
}
```

`UseCaseEvent` exposes:

- `name()`: the event name.
- `request()`: the JSON-RPC request snapshot, including `jsonrpc`, `method`, optional `params`, and optional `id`.
- `input()`: the normalized use-case input payload as `serde_json::Value`.
- `output()`: `None` for `Will*` events and `Some(value)` for `Did*` events.

Event payload values use the same JSON casing as JSON-RPC requests and responses. `Will*` consumers run synchronously before the use case executes. `Did*` consumers are scheduled after the JSON-RPC response value or string has been constructed and run on a detached temporary thread, so they do not delay the response path. Protocol validation failures and invalid params publish no use-case events. Use-case errors publish `Will*`, but not `Did*`.

## JSON Field Case

JSON-RPC method names stay PascalCase:

```json
{ "method": "AddNumbers" }
```

Request `params` are expected in camelCase:

```json
{
  "leftOperand": 2,
  "rightOperand": 3
}
```

The library converts camelCase params into Rust snake_case before deserializing `Input`:

```rust,ignore
#[derive(Deserialize)]
struct AddNumbersInput {
    left_operand: i64,
    right_operand: i64,
}
```

Successful outputs and error data are converted back to camelCase before being returned as JSON.

## Results And Errors

A successful `Output` is serialized into `result`:

```json
{
  "jsonrpc": "2.0",
  "result": { "computedSum": 5 },
  "id": 1
}
```

A use-case error is converted into a JSON-RPC error object:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": 10001,
    "message": "AddNumbersError",
    "data": { "failureReason": "only positive numbers are accepted" }
  },
  "id": 1
}
```

The error mapping is controlled by the library `Error` trait:

- `code()` is required and becomes `error.code`.
- `message()` defaults to the Rust error type name and can be overridden.
- `data()` defaults to the serialized error value and can be overridden.

Example with a custom message:

```rust,ignore
impl Error for AddNumbersError {
    fn code(&self) -> i64 {
        10_001
    }

    fn message(&self) -> std::borrow::Cow<'static, str> {
        "InvalidAddNumbersInput".into()
    }
}
```

## JSON-RPC Errors Handled By The Library

The library handles protocol errors before dispatching to your use cases:

- `-32700`: parse error
- `-32600`: invalid request
- `-32601`: method not found
- `-32602`: invalid params
- `-32603`: internal error while serializing a successful use-case output

The response types are internal. Match on the serialized JSON if tests need to assert protocol behavior.

## Batches

Batch requests are supported. Each item is dispatched concurrently and the response order follows the request order.

```rust,ignore
use serde_json::json;

let response = service.handle_value(json!([
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
    }
])).await;

assert_eq!(response, Some(json!([
    {
        "jsonrpc": "2.0",
        "result": { "computedSum": 5 },
        "id": "first"
    }
])));
```

The second item is a notification because it has no `id`, so it is executed but omitted from the batch response.

## Axum Adapter

Enable the `axum` feature to get a ready-to-use router for the configured endpoint:

```rust,ignore
use jsonrpc_usecase::{JsonRpcService, axum};

let service = JsonRpcService::builder()
    .endpoint("/rpc")
    .build()?;

let app = axum::router(service);
```

The adapter:

- registers one `POST` route at `service.endpoint()`
- passes the raw body to `JsonRpcService::handle_json`
- returns `application/json` for normal responses
- returns `204 No Content` for notification-only requests

Without the `axum` feature, wire `JsonRpcService` into any HTTP framework manually:

```rust,ignore
use jsonrpc_usecase::JsonRpcService;

async fn rpc_handler(service: JsonRpcService, body: String) -> Option<String> {
    service.handle_json(&body).await
}
```
