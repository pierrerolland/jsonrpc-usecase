use futures::executor::block_on;
use jsonrpc_usecase::{
    Error, InputValidationErrors, JsonRpcService, UseCase, UseCaseEvent, UseCaseEventConsumer,
    UseCaseExecutionError, UseCaseInput,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fmt::{self, Display, Formatter},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

static VALID_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);
static INVALID_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);
static INVALID_WILL_EVENTS: AtomicUsize = AtomicUsize::new(0);
static DIRECT_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);
static NESTED_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);
static NESTED_CHILD_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);
static OBSERVED_WILL_INPUT: Mutex<Option<serde_json::Value>> = Mutex::new(None);

#[derive(Debug, Deserialize, UseCaseInput)]
struct AddressInput {
    #[transform(trim, collapse_whitespace)]
    #[validate(not_blank, length(min = 3, max = 80))]
    street: String,
}

#[derive(Deserialize, UseCaseInput)]
struct CreateValidatedProfileInput {
    #[transform(trim, lowercase)]
    #[validate(not_blank, email, length(max = 254))]
    email: String,

    #[transform(trim, collapse_whitespace, titlecase)]
    #[validate(not_blank, length(min = 2, max = 40))]
    display_name: String,

    #[validate(range(min = 18, max = 120))]
    age: u8,

    #[validate(required)]
    nickname: Option<String>,

    #[transform(each(trim, lowercase), sort, dedup)]
    #[validate(length(min = 1, max = 4), unique, each(not_blank, length(max = 12)))]
    tags: Vec<String>,

    #[validate(nested)]
    address: AddressInput,
}

#[derive(Serialize)]
struct CreateValidatedProfileOutput {
    email: String,
    display_name: String,
    age: u8,
    nickname: Option<String>,
    tags: Vec<String>,
    street: String,
}

#[derive(Debug, Serialize)]
struct ProfileError;

impl Display for ProfileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("profile error")
    }
}

impl std::error::Error for ProfileError {}

impl Error for ProfileError {
    fn code(&self) -> i64 {
        20_001
    }
}

#[derive(Default)]
struct NormalizeStreet;

#[UseCase]
impl NormalizeStreet {
    async fn execute(&self, input: AddressInput) -> Result<String, ProfileError> {
        DIRECT_EXECUTIONS.fetch_add(1, Ordering::SeqCst);
        if input.street == "execution error" {
            return Err(ProfileError);
        }
        Ok(input.street)
    }
}

#[derive(Default)]
struct NormalizeStreetForNesting;

#[UseCase]
impl NormalizeStreetForNesting {
    async fn execute(&self, input: AddressInput) -> Result<String, ProfileError> {
        NESTED_CHILD_EXECUTIONS.fetch_add(1, Ordering::SeqCst);
        Ok(input.street)
    }
}

#[derive(Deserialize)]
struct NormalizeStreetFromUseCaseInput {
    street: String,
}

#[derive(Debug, Serialize)]
enum NestedProfileError {
    InvalidInput(InputValidationErrors),
    Execution(ProfileError),
}

impl From<UseCaseExecutionError<ProfileError>> for NestedProfileError {
    fn from(error: UseCaseExecutionError<ProfileError>) -> Self {
        match error {
            UseCaseExecutionError::InvalidInput(errors) => Self::InvalidInput(errors),
            UseCaseExecutionError::Execution(error) => Self::Execution(error),
        }
    }
}

impl Display for NestedProfileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(errors) => Display::fmt(errors, formatter),
            Self::Execution(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for NestedProfileError {}

impl Error for NestedProfileError {
    fn code(&self) -> i64 {
        20_002
    }
}

#[derive(Default)]
struct NormalizeStreetFromUseCase;

#[UseCase]
impl NormalizeStreetFromUseCase {
    async fn execute(
        &self,
        input: NormalizeStreetFromUseCaseInput,
    ) -> Result<String, NestedProfileError> {
        NESTED_EXECUTIONS.fetch_add(1, Ordering::SeqCst);
        NormalizeStreetForNesting
            .execute(AddressInput {
                street: input.street,
            })
            .await
            .map_err(Into::into)
    }
}

#[derive(Default)]
struct CreateValidatedProfile;

#[UseCaseEventConsumer(event = "WillCreateValidatedProfile")]
async fn observe_validated_profile_input(event: &UseCaseEvent) {
    if event.request().id() != Some(&json!("valid-profile")) {
        return;
    }

    let typed = event
        .get_input::<CreateValidatedProfileInput>()
        .expect("Will event carries the typed input");
    *OBSERVED_WILL_INPUT.lock().unwrap() = Some(json!({
        "rawEmail": event.input()["email"],
        "typedEmail": typed.email.as_str(),
        "typedStreet": typed.address.street.as_str(),
        "typedTags": typed.tags.as_slice(),
    }));
}

#[UseCase]
impl CreateValidatedProfile {
    async fn execute(
        &self,
        input: CreateValidatedProfileInput,
    ) -> Result<CreateValidatedProfileOutput, ProfileError> {
        VALID_EXECUTIONS.fetch_add(1, Ordering::SeqCst);
        Ok(CreateValidatedProfileOutput {
            email: input.email,
            display_name: input.display_name,
            age: input.age,
            nickname: input.nickname,
            tags: input.tags,
            street: input.address.street,
        })
    }
}

#[derive(Default)]
struct NeverExecuteInvalidProfile;

#[UseCaseEventConsumer(event = "WillNeverExecuteInvalidProfile")]
async fn count_invalid_profile_will_events(_event: &UseCaseEvent) {
    INVALID_WILL_EVENTS.fetch_add(1, Ordering::SeqCst);
}

#[UseCase]
impl NeverExecuteInvalidProfile {
    async fn execute(&self, _input: CreateValidatedProfileInput) -> Result<(), ProfileError> {
        INVALID_EXECUTIONS.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn service() -> JsonRpcService {
    JsonRpcService::builder().build().unwrap()
}

#[test]
fn transforms_before_validation_and_execution() {
    VALID_EXECUTIONS.store(0, Ordering::SeqCst);
    *OBSERVED_WILL_INPUT.lock().unwrap() = None;

    let response = block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "CreateValidatedProfile",
        "params": {
            "email": "  PERSON@EXAMPLE.COM ",
            "displayName": "  jEAN   lUC ",
            "age": 32,
            "nickname": "P",
            "tags": [" Rust ", "RPC", "rust"],
            "address": { "street": "  10   Main Street " }
        },
        "id": "valid-profile"
    })));

    assert_eq!(
        response,
        Some(json!({
            "jsonrpc": "2.0",
            "result": {
                "email": "person@example.com",
                "displayName": "Jean Luc",
                "age": 32,
                "nickname": "P",
                "tags": ["rpc", "rust"],
                "street": "10 Main Street"
            },
            "id": "valid-profile"
        }))
    );
    assert_eq!(VALID_EXECUTIONS.load(Ordering::SeqCst), 1);
    assert_eq!(
        *OBSERVED_WILL_INPUT.lock().unwrap(),
        Some(json!({
            "rawEmail": "  PERSON@EXAMPLE.COM ",
            "typedEmail": "person@example.com",
            "typedStreet": "10 Main Street",
            "typedTags": ["rpc", "rust"],
        }))
    );
}

#[test]
fn returns_all_violations_and_does_not_execute() {
    INVALID_EXECUTIONS.store(0, Ordering::SeqCst);
    INVALID_WILL_EVENTS.store(0, Ordering::SeqCst);

    let response = block_on(service().handle_value(json!({
        "jsonrpc": "2.0",
        "method": "NeverExecuteInvalidProfile",
        "params": {
            "email": "bad",
            "displayName": " ",
            "age": 12,
            "nickname": null,
            "tags": ["valid", " "],
            "address": { "street": " " }
        },
        "id": "invalid-profile"
    })))
    .unwrap();

    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["message"], "Invalid params");
    assert_eq!(
        response["error"]["data"]["violations"],
        json!([
            {
                "field": "email",
                "rule": "email",
                "message": "must be a valid email address"
            },
            {
                "field": "displayName",
                "rule": "not_blank",
                "message": "must not be blank"
            },
            {
                "field": "displayName",
                "rule": "length",
                "message": "must contain at least 2 item(s)"
            },
            {
                "field": "age",
                "rule": "range",
                "message": "must be within the inclusive range"
            },
            {
                "field": "nickname",
                "rule": "required",
                "message": "is required"
            },
            {
                "field": "tags[0]",
                "rule": "not_blank",
                "message": "must not be blank"
            },
            {
                "field": "address.street",
                "rule": "not_blank",
                "message": "must not be blank"
            },
            {
                "field": "address.street",
                "rule": "length",
                "message": "must contain at least 3 item(s)"
            }
        ])
    );
    assert_eq!(INVALID_EXECUTIONS.load(Ordering::SeqCst), 0);
    assert_eq!(INVALID_WILL_EVENTS.load(Ordering::SeqCst), 0);
}

fn prepare_internal_call(input: AddressInput) -> Result<AddressInput, InputValidationErrors> {
    input.into_processed()
}

#[test]
fn input_violations_propagate_without_jsonrpc() {
    let input = AddressInput {
        street: "  Rue   de Rivoli ".to_owned(),
    };

    let mut input = prepare_internal_call(input).unwrap();
    assert_eq!(input.street, "Rue de Rivoli");

    input.street = " ".to_owned();
    let errors: InputValidationErrors = prepare_internal_call(input).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors.violations()[0].field(), "street");
}

#[test]
fn direct_use_case_execution_raises_input_violations() {
    DIRECT_EXECUTIONS.store(0, Ordering::SeqCst);
    let use_case = NormalizeStreet;

    let failure = block_on(use_case.execute(AddressInput {
        street: " ".to_owned(),
    }))
    .unwrap_err();

    let UseCaseExecutionError::InvalidInput(errors) = failure else {
        panic!("expected an invalid-input error");
    };
    assert_eq!(errors.len(), 2);
    assert_eq!(DIRECT_EXECUTIONS.load(Ordering::SeqCst), 0);

    let street = block_on(use_case.execute(AddressInput {
        street: "  Rue   de Rivoli ".to_owned(),
    }))
    .unwrap();
    assert_eq!(street, "Rue de Rivoli");
    assert_eq!(DIRECT_EXECUTIONS.load(Ordering::SeqCst), 1);

    let failure = block_on(use_case.execute(AddressInput {
        street: " execution   error ".to_owned(),
    }))
    .unwrap_err();
    assert!(matches!(
        failure,
        UseCaseExecutionError::Execution(ProfileError)
    ));
    assert_eq!(DIRECT_EXECUTIONS.load(Ordering::SeqCst), 2);
}

#[test]
fn nested_use_cases_call_the_same_validated_execute_method() {
    NESTED_EXECUTIONS.store(0, Ordering::SeqCst);
    NESTED_CHILD_EXECUTIONS.store(0, Ordering::SeqCst);

    let failure = block_on(
        NormalizeStreetFromUseCase.execute(NormalizeStreetFromUseCaseInput {
            street: " ".to_owned(),
        }),
    )
    .unwrap_err();

    let UseCaseExecutionError::Execution(NestedProfileError::InvalidInput(errors)) = failure else {
        panic!("expected the nested input violation to propagate");
    };
    assert_eq!(errors.len(), 2);
    assert_eq!(NESTED_EXECUTIONS.load(Ordering::SeqCst), 1);
    assert_eq!(NESTED_CHILD_EXECUTIONS.load(Ordering::SeqCst), 0);

    let street = block_on(
        NormalizeStreetFromUseCase.execute(NormalizeStreetFromUseCaseInput {
            street: "  Rue   de Rivoli ".to_owned(),
        }),
    )
    .unwrap();

    assert_eq!(street, "Rue de Rivoli");
    assert_eq!(NESTED_EXECUTIONS.load(Ordering::SeqCst), 2);
    assert_eq!(NESTED_CHILD_EXECUTIONS.load(Ordering::SeqCst), 1);
}
