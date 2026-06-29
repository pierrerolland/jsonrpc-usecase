use crate::{event::UseCaseEvent, method::RpcMethod};
use std::sync::Arc;

pub struct UseCaseRegistration {
    pub method: &'static str,
    pub factory: fn() -> Arc<dyn RpcMethod>,
}

inventory::collect!(UseCaseRegistration);

pub struct UseCaseEventConsumerRegistration {
    pub event: &'static str,
    pub consumer: fn(&UseCaseEvent),
}

inventory::collect!(UseCaseEventConsumerRegistration);
