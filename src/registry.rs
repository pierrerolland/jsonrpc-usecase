use crate::{event::UseCaseEvent, method::RpcMethod};
use std::{future::Future, pin::Pin, sync::Arc};

pub struct UseCaseRegistration {
    pub method: &'static str,
    pub factory: fn() -> Arc<dyn RpcMethod>,
}

inventory::collect!(UseCaseRegistration);

pub struct UseCaseEventConsumerRegistration {
    pub event: &'static str,
    pub consumer: for<'a> fn(&'a UseCaseEvent) -> UseCaseEventConsumerFuture<'a>,
}

pub type UseCaseEventConsumerFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

inventory::collect!(UseCaseEventConsumerRegistration);
