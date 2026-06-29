extern crate self as jsonrpc_usecase;

mod case;
mod config;
mod error;
mod event;
mod method;
mod registry;
mod request;
mod response;
mod service;
mod use_case;

#[cfg(feature = "axum")]
pub mod axum;

pub use error::Error;
pub use event::{EventRequest, UseCaseEvent};
pub use jsonrpc_usecase_macros::{UseCase, UseCaseEventConsumer};
pub use service::{JsonRpcService, JsonRpcServiceBuilder, RegistrationError};

pub(crate) const JSONRPC_VERSION: &str = "2.0";

#[doc(hidden)]
pub mod __private {
    pub use crate::{
        method::{RpcMethod, UseCaseMethod},
        registry::{UseCaseEventConsumerRegistration, UseCaseRegistration},
        use_case::UseCaseDefinition,
    };
    pub use inventory;
}

pub mod prelude {
    pub use crate::{
        Error, EventRequest, JsonRpcService, UseCase, UseCaseEvent, UseCaseEventConsumer,
    };
}
