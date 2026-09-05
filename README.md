# jsonrpc-usecase

`jsonrpc-usecase` turns Rust use cases into JSON-RPC 2.0 methods with a single macro.

The intended workflow is:

1. Define a struct for the use case.
2. Implement an inherent `async fn execute(&self, input) -> Result<output, error>` method.
3. Put `#[UseCase]` on that `impl` block.
4. Build a `JsonRpcService`; use cases with JSON-RPC enabled are discovered automatically.

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
- `UseCaseInput`: derive macro and trait for use-case input transformations and validation.
- `InputValidationErrors` and `InputViolation`: structured validation results for direct use.
- `UseCaseExecutionError`: distinguishes invalid input from a use case's execution error.
- `UseCaseEventConsumer`: attribute macro applied to an event consumer struct, function, or impl block.
- `Error`: trait implemented by application error types.
- `Guard`: trait implemented by access-control guard types.
- `JsonRpcService`: framework-neutral JSON-RPC handler.
- `JsonRpcServiceBuilder`: builder returned by `JsonRpcService::builder()`.
- `EventRequest` and `UseCaseEvent`: event payload types.
- `GuardContext` and `RequestHeaders`: guard payload types.
- `ContextBuilderRequest`, `RequestContext`, `current_context`, and `with_current_context`: request context APIs.
- `RegistrationError`: returned when the auto-registration registry is invalid, for example duplicate method names.

The JSON-RPC request parser, response DTOs, dispatcher, registry, and macro support module are internal. Treat responses as JSON returned by `JsonRpcService`.

## Install

```toml
[dependencies]
jsonrpc-usecase = "0.7.0"
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
jsonrpc-usecase = { version = "0.7.0", features = ["axum"] }
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

The macro also implements the hidden runtime trait and, by default, submits the use case to the global JSON-RPC registry.

Use-case structs with JSON-RPC registration enabled must implement `Default`, because the service constructs their instances automatically.

## Disable JSON-RPC Registration

Set `jsonrpc = false` for a use case that should only be called from Rust:

```rust,ignore
struct InternalAddNumbers;

#[UseCase(jsonrpc = false)]
impl InternalAddNumbers {
    async fn execute(&self, input: AddNumbersInput) -> Result<AddNumbersOutput, AddNumbersError> {
        Ok(AddNumbersOutput {
            computed_sum: input.left_operand + input.right_operand,
        })
    }
}

let output = InternalAddNumbers.execute(input).await?;
```

The use case keeps its generated `execute` method, including input transformations, validation, and `UseCaseExecutionError` handling. It can be called directly or from another use case and does not need to implement `Default`. The existing input, output, and error type requirements still apply.

No JSON-RPC method is registered for it, even if `method = "..."` is also supplied. Requests for an unregistered method return `Method not found` (`-32601`). Registration is enabled by default; `#[UseCase(jsonrpc = true)]` makes that explicit.

## Validate And Transform Inputs

Derive `UseCaseInput` on any use-case input that needs preprocessing. Put transformations and validators on its fields:

```rust,ignore
use jsonrpc_usecase::{UseCase, UseCaseInput};
use serde::Deserialize;

#[derive(Deserialize, UseCaseInput)]
struct CreateAccountInput {
    #[transform(trim, lowercase)]
    #[validate(not_blank, email, length(max = 254))]
    email: String,

    #[transform(trim, collapse_whitespace)]
    #[validate(not_blank, length(min = 2, max = 80))]
    display_name: String,

    #[validate(range(min = 18, max = 120))]
    age: u8,

    #[validate(required)]
    referral_code: Option<String>,

    #[transform(each(trim, lowercase), sort, dedup)]
    #[validate(length(max = 10), unique, each(not_blank, length(max = 24)))]
    tags: Vec<String>,
}

#[derive(Default)]
struct CreateAccount;

#[UseCase]
impl CreateAccount {
    async fn execute(
        &self,
        input: CreateAccountInput,
    ) -> Result<CreateAccountOutput, CreateAccountError> {
        // The generated outer `execute` guarantees prepared input here.
        todo!()
    }
}
```

No option is needed on `#[UseCase]`. When the use case is exposed as JSON-RPC, a derived input is discovered automatically. Existing inputs that only derive `Deserialize` keep their current behavior.

Processing has these guarantees:

- Every transformation runs, in declaration order, before any validator.
- Validation happens before the developer-authored `execute` body; JSON-RPC performs it after deserialization and before `Will*` events.
- All field violations are collected; validation does not stop at the first failure.
- Validators skip `None`. Add `required` when an `Option<T>` must contain a value.
- `length` counts Unicode scalar values for strings and elements for collections, not UTF-8 bytes.
- `each(...)` applies its rules to every `Vec` or array element and reports paths such as `tags[2]`.
- A custom message can be set with list syntax, for example `email(message = "enter a valid address")`.

Validation failures use the standard JSON-RPC invalid-params code and do not execute the use case:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": {
      "violations": [
        {
          "field": "displayName",
          "rule": "length",
          "message": "must contain at least 2 item(s)"
        }
      ]
    }
  },
  "id": 1
}
```

Validation belongs to the use-case input, not to JSON-RPC. `#[UseCase]` turns the developer-defined `execute` body into the prepared implementation and exposes an outer `execute(input)` method that always transforms and validates first:

```rust,ignore
use jsonrpc_usecase::UseCaseExecutionError;

match CreateAccount::default().execute(input).await {
    Ok(output) => { /* use output */ }
    Err(UseCaseExecutionError::InvalidInput(violations)) => {
        // Inspect or propagate every InputViolation.
    }
    Err(UseCaseExecutionError::Execution(error)) => {
        // Handle CreateAccountError.
    }
}
```

Use `execute` in every context, including when nesting one use case inside another. At call sites it returns `UseCaseExecutionError<YourError>`. A parent use case can implement `From<UseCaseExecutionError<ChildError>>` for its own error and propagate with `?`, or map the error explicitly. Only the JSON-RPC adapter translates `InvalidInput` into an invalid-params response; direct callers receive the violation normally.

Inputs can also be prepared without executing their use case. Calling `process()` or `into_processed()` returns `InputValidationErrors` directly:

```rust,ignore
use jsonrpc_usecase::{InputValidationErrors, UseCaseInput};

fn prepare_internal_call(
    input: CreateAccountInput,
) -> Result<CreateAccountInput, InputValidationErrors> {
    input.into_processed()
}
```

`InputValidationErrors` also implements `std::error::Error`. For an existing mutable value, the two processing phases are public:

```rust,ignore
input.process()?; // transform, then validate

// Or invoke the two phases separately:
input.transform();
input.validate()?;
```

### Validator Catalog

Rules without arguments use their bare name. Single-value rules support `rule = value`; rules with several arguments use `rule(name = value, ...)`.

| Category | Rule | Applies to / behavior |
| --- | --- | --- |
| Presence | `required` | `Option<T>` must be `Some` |
| Size | `not_empty` | String or collection has at least one item |
| Size | `not_blank` | String contains a non-whitespace character |
| Size | `length(min = n, max = n)` | Inclusive string/collection length; either bound may be omitted |
| Size | `length(exact = n)` | Exact string/collection length |
| String | `ascii` | Contains only ASCII characters |
| String | `alphabetic` | Non-empty and all characters are alphabetic |
| String | `alphanumeric` | Non-empty and all characters are alphabetic or numeric |
| String | `numeric` | Non-empty and all characters are numeric |
| String | `lowercase`, `uppercase` | All cased characters use the requested case |
| String | `contains = "x"` | Contains a substring |
| String | `starts_with = "x"`, `ends_with = "x"` | Has the requested boundary text |
| Format | `email` | ASCII mailbox plus a qualified hostname or bracketed IP address |
| Format | `url` | Absolute URL accepted by the URL standard parser |
| Format | `regex = "..."` | Matches a cached regular expression |
| Format | `uuid` | UUID accepted by the UUID parser |
| Format | `ip`, `ipv4`, `ipv6` | IP address of the requested kind |
| Format | `hostname` | Valid ASCII hostname labels and lengths |
| Format | `slug` | Lowercase ASCII letters, digits, and single interior hyphens |
| Format | `hex` | Non-empty, even-length hexadecimal text |
| Format | `base64`, `base64_url` | Standard or URL-safe Base64; URL-safe accepts padded or unpadded input |
| Number | `min = n`, `max = n` | Inclusive minimum or maximum |
| Number | `range(min = n, max = n)` | Inclusive range |
| Number | `positive`, `negative` | Strict sign check |
| Number | `non_positive`, `non_negative` | Inclusive sign check |
| Number | `finite` | Finite `f32` or `f64` value |
| Number | `integer` | Integer primitive, or float with no fractional part |
| Number | `multiple_of = n` | Integer or floating-point multiple |
| Number | `even`, `odd` | Integer parity |
| Equality | `equals = value`, `not_equals = value` | Equality check using the field type |
| Equality | `one_of = [a, b, c]` | Value appears in the configured set |
| Collection | `unique` | `Vec` or array contains no duplicates |
| Collection | `each(rule, ...)` | Applies validators to every element |
| Object | `nested` | Runs another derived `UseCaseInput` and prefixes its field paths |
| Extension | `custom = function` | Calls `fn(&T) -> Result<(), E>` where `E: Display`; normal deref coercions apply |

For nested inputs, derive `UseCaseInput` on both structs and mark the child field:

```rust,ignore
#[derive(Deserialize, UseCaseInput)]
struct AddressInput {
    #[transform(trim)]
    #[validate(not_blank)]
    street: String,
}

#[derive(Deserialize, UseCaseInput)]
struct CreateAccountInput {
    #[validate(nested)]
    address: AddressInput,

    #[validate(each(nested))]
    previous_addresses: Vec<AddressInput>,
}
```

`nested` also applies the child transformations before validating it.

### Transformer Catalog

| Category | Rule | Behavior |
| --- | --- | --- |
| Whitespace | `trim`, `trim_start`, `trim_end` | Removes Unicode whitespace at the requested boundary |
| Whitespace | `collapse_whitespace` | Trims and replaces each whitespace run with one ASCII space |
| Whitespace | `remove_whitespace` | Removes every Unicode whitespace character |
| Case | `lowercase`, `uppercase` | Applies Unicode case conversion |
| Case | `capitalize` | Uppercases the first character |
| Case | `titlecase` | Uppercases each word start and lowercases its remaining characters |
| String | `replace(from = "x", to = "y")` | Replaces every matching substring |
| String | `strip_prefix = "x"`, `strip_suffix = "x"` | Removes one matching prefix or suffix |
| String | `truncate = n` | Keeps at most `n` Unicode scalar values |
| Unicode | `normalize_nfc`, `normalize_nfkc` | Applies canonical or compatibility normalization |
| Number | `clamp(min = n, max = n)` | Clamps to an inclusive range |
| Number | `abs` | Absolute value for signed integers and floats |
| Number | `round(precision = n)` | Rounds a float; `precision` defaults to zero |
| Number | `floor`, `ceil` | Rounds a float down or up |
| Collection | `sort` | Sorts a `Vec` or array using `Ord` |
| Collection | `dedup` | Removes consecutive duplicate `Vec` values; use after `sort` for global deduplication |
| Collection | `each(rule, ...)` | Applies transformers to every element |
| Extension | `custom = function` | Calls `fn(&mut T)` |

Custom validators use `custom(function = path, name = "rule_name")` when the violation should have a rule name other than `custom`. They also support `message = "..."` like built-in validators.

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

## Request Context

You can build one typed context value per incoming HTTP request and make it available to guards, use cases, and event consumers. The proc macro does not change the `execute(&self, input)` signature; use cases read the current context from the runtime scope.

```rust,ignore
use jsonrpc_usecase::{JsonRpcService, current_context};

#[derive(Clone)]
struct CallerContext {
    user_id: Option<String>,
    role: String,
    trace_id: Option<String>,
}

let service = JsonRpcService::builder()
    .endpoint("/rpc")
    .context_builder(|request| CallerContext {
        user_id: request.headers().get("x-user-id").map(str::to_owned),
        role: request.headers().get("x-role").unwrap_or("guest").to_owned(),
        trace_id: request.headers().get("x-trace-id").map(str::to_owned),
    })
    .build()?;
```

Inside a use case:

```rust,ignore
#[UseCase]
impl ReadAccount {
    async fn execute(&self, input: ReadAccountInput) -> Result<ReadAccountOutput, ReadAccountError> {
        let caller = current_context::<CallerContext>()
            .expect("CallerContext was configured on the service");

        if caller.user_id.as_deref() != Some(input.account_owner_id.as_str()) {
            return Err(ReadAccountError::forbidden());
        }

        todo!()
    }
}
```

Guards receive the same context through `GuardContext`:

```rust,ignore
impl Guard for RequireAdmin {
    fn can_proceed(&self, context: &GuardContext) -> bool {
        context
            .get_context::<CallerContext>()
            .is_some_and(|caller| caller.role == "admin")
    }
}
```

Event consumers receive it through `UseCaseEvent`:

```rust,ignore
impl AuditReadAccount {
    async fn consume(&self, event: &UseCaseEvent) {
        let caller = event.get_context::<CallerContext>();
        let request = event.request();
        let output = event.output();
    }
}
```

Use `async_context_builder` when building the context needs async work, for example loading a session from a database. In JSON-RPC batches, the context builder runs once for the HTTP request and the resulting context is shared by every item in the batch.

## Build The Service

All `#[UseCase]` impl blocks linked into the binary are auto-registered when the service is built, except those marked `#[UseCase(jsonrpc = false)]`.

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
    async fn consume(&self, event: &UseCaseEvent) {
        let method = event.request().method();
        let input = event
            .get_input::<AddNumbersInput>()
            .expect("WillAddNumbers carries AddNumbersInput");
    }
}

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct AuditAddNumbersResult;

impl AuditAddNumbersResult {
    async fn consume(&self, event: &UseCaseEvent) {
        let request_id = event.request().id();
        let output = event
            .get_output::<AddNumbersOutput>()
            .expect("DidAddNumbers carries AddNumbersOutput");
    }
}
```

All linked event consumers are auto-discovered. There is no service builder registration step. Struct consumers must implement `Default` and have an async `consume(&self, event: &UseCaseEvent)` method.

Multiple consumers can listen to the same event by using the same event name more than once:

```rust,ignore
use jsonrpc_usecase::{UseCaseEvent, UseCaseEventConsumer};

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct WriteAddNumbersAuditLog;

impl WriteAddNumbersAuditLog {
    async fn consume(&self, event: &UseCaseEvent) {
        println!("audit event: {}", event.name());
    }
}

#[UseCaseEventConsumer(event = "DidAddNumbers")]
#[derive(Default)]
struct UpdateAddNumbersMetrics;

impl UpdateAddNumbersMetrics {
    async fn consume(&self, event: &UseCaseEvent) {
        println!("metric event: {}", event.name());
    }
}
```

`UseCaseEvent` exposes:

- `name()`: the event name.
- `request()`: the JSON-RPC request snapshot, including `jsonrpc`, `method`, optional `params`, and optional `id`.
- `context()`: the typed request context wrapper built by the service.
- `input()`: the normalized original input payload as `serde_json::Value`.
- `output()`: `None` for `Will*` events and `Some(value)` for `Did*` events.
- `get_context::<T>()`: the typed request context when `T` is the configured context type.
- `get_input::<T>()`: the typed input payload when `T` is the use-case input type.
- `get_output::<T>()`: the typed output payload for `Did*` events when `T` is the use-case output type.

Raw event payload values use the same JSON casing as JSON-RPC requests and responses. `input()` preserves the normalized request value, while `get_input::<T>()` returns the transformed, validated Rust input when `T` derives `UseCaseInput`. Typed payload getters do not go through `serde_json::Value`. `Will*` consumers are awaited before the use case executes. `Did*` consumers are scheduled after the JSON-RPC response value or string has been constructed and run on a shared background Tokio runtime, so async consumers do not need to create their own runtime and do not delay the response path. Protocol validation failures and invalid params publish no use-case events. Use-case errors publish `Will*`, but not `Did*`.

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
