use crate::{JSONRPC_VERSION, registry::UseCaseEventConsumerRegistration, request::ValidRequest};
use serde::Serialize;
use serde_json::Value;
use std::{
    any::Any,
    fmt::{self, Debug, Formatter},
    sync::{Arc, OnceLock},
};
use tokio::runtime::{Builder, Runtime};

type TypedPayload = Arc<dyn Any + Send + Sync>;

pub(crate) async fn publish(event: &UseCaseEvent) {
    for registration in inventory::iter::<UseCaseEventConsumerRegistration> {
        if registration.event == event.name() {
            (registration.consumer)(event).await;
        }
    }
}

pub(crate) fn publish_async(event: UseCaseEvent) {
    event_runtime().spawn(async move {
        publish(&event).await;
    });
}

fn event_runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .enable_all()
            .thread_name("jsonrpc-usecase-event")
            .build()
            .expect("event runtime should be created")
    })
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EventRequest {
    jsonrpc: &'static str,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
}

impl EventRequest {
    pub(crate) fn from_valid_request(request: &ValidRequest) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            method: request.method.clone(),
            params: request.params.clone(),
            id: request.id.clone(),
        }
    }

    pub fn jsonrpc(&self) -> &'static str {
        self.jsonrpc
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn params(&self) -> Option<&Value> {
        self.params.as_ref()
    }

    pub fn id(&self) -> Option<&Value> {
        self.id.as_ref()
    }
}

#[derive(Clone)]
pub struct UseCaseEvent {
    name: &'static str,
    request: EventRequest,
    input: Value,
    output: Option<Value>,
    typed_input: Option<TypedPayload>,
    typed_output: Option<TypedPayload>,
}

impl UseCaseEvent {
    pub(crate) fn will_typed<Input>(
        name: &'static str,
        request: EventRequest,
        input: Value,
        typed_input: Arc<Input>,
    ) -> Self
    where
        Input: Send + Sync + 'static,
    {
        let typed_input: TypedPayload = typed_input;

        Self {
            name,
            request,
            input,
            output: None,
            typed_input: Some(typed_input),
            typed_output: None,
        }
    }

    pub(crate) fn did_typed<Input, Output>(
        name: &'static str,
        request: EventRequest,
        input: Value,
        output: Value,
        typed_input: Arc<Input>,
        typed_output: Output,
    ) -> Self
    where
        Input: Send + Sync + 'static,
        Output: Send + Sync + 'static,
    {
        let typed_input: TypedPayload = typed_input;

        Self {
            name,
            request,
            input,
            output: Some(output),
            typed_input: Some(typed_input),
            typed_output: Some(Arc::new(typed_output)),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn request(&self) -> &EventRequest {
        &self.request
    }

    pub fn input(&self) -> &Value {
        &self.input
    }

    pub fn get_input<Input>(&self) -> Option<&Input>
    where
        Input: 'static,
    {
        self.typed_input.as_ref()?.as_ref().downcast_ref()
    }

    pub fn output(&self) -> Option<&Value> {
        self.output.as_ref()
    }

    pub fn get_output<Output>(&self) -> Option<&Output>
    where
        Output: 'static,
    {
        self.typed_output.as_ref()?.as_ref().downcast_ref()
    }
}

impl Debug for UseCaseEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UseCaseEvent")
            .field("name", &self.name)
            .field("request", &self.request)
            .field("input", &self.input)
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

impl PartialEq for UseCaseEvent {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.request == other.request
            && self.input == other.input
            && self.output == other.output
    }
}
