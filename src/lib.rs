extern crate self as jsonrpc_usecase;

mod case;
mod config;
mod context;
mod error;
mod event;
mod guard;
mod method;
mod registry;
mod request;
mod response;
mod service;
mod use_case;

#[cfg(feature = "axum")]
pub mod axum;

pub use context::{ContextBuilderRequest, RequestContext, current_context, with_current_context};
pub use error::Error;
pub use event::{EventRequest, UseCaseEvent};
pub use guard::{Guard, GuardContext, RequestHeader, RequestHeaders};
pub use jsonrpc_usecase_macros::{UseCase, UseCaseEventConsumer};
pub use service::{JsonRpcService, JsonRpcServiceBuilder, RegistrationError};

pub(crate) const JSONRPC_VERSION: &str = "2.0";

#[doc(hidden)]
pub mod __private {
    pub use crate::{
        guard::{Guard, GuardContext},
        method::{RpcMethod, UseCaseMethod},
        registry::{
            UseCaseEventConsumerFuture, UseCaseEventConsumerRegistration, UseCaseRegistration,
        },
        use_case::UseCaseDefinition,
    };
    pub use inventory;
}

pub mod prelude {
    pub use crate::{
        ContextBuilderRequest, Error, EventRequest, Guard, GuardContext, JsonRpcService,
        RequestContext, RequestHeader, RequestHeaders, UseCase, UseCaseEvent, UseCaseEventConsumer,
        current_context, with_current_context,
    };
}
