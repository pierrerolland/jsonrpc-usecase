use crate::method::RpcMethod;
use std::sync::Arc;

pub struct UseCaseRegistration {
    pub method: &'static str,
    pub factory: fn() -> Arc<dyn RpcMethod>,
}

inventory::collect!(UseCaseRegistration);
