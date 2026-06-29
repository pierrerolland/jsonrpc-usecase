use crate::{JSONRPC_VERSION, registry::UseCaseEventConsumerRegistration, request::ValidRequest};
use serde::Serialize;
use serde_json::Value;
use std::thread;

pub(crate) fn publish(event: &UseCaseEvent) {
    for registration in inventory::iter::<UseCaseEventConsumerRegistration> {
        if registration.event == event.name() {
            (registration.consumer)(event);
        }
    }
}

pub(crate) fn publish_async(event: UseCaseEvent) {
    thread::spawn(move || publish(&event));
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

#[derive(Clone, Debug, PartialEq)]
pub struct UseCaseEvent {
    name: &'static str,
    request: EventRequest,
    input: Value,
    output: Option<Value>,
}

impl UseCaseEvent {
    pub(crate) fn will(name: &'static str, request: EventRequest, input: Value) -> Self {
        Self {
            name,
            request,
            input,
            output: None,
        }
    }

    pub(crate) fn did(
        name: &'static str,
        request: EventRequest,
        input: Value,
        output: Value,
    ) -> Self {
        Self {
            name,
            request,
            input,
            output: Some(output),
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

    pub fn output(&self) -> Option<&Value> {
        self.output.as_ref()
    }
}
