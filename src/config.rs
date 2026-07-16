use crate::context::{
    ContextBuilder, ContextBuilderRequest, RequestContext, empty_context_builder,
};

#[derive(Clone)]
pub(crate) struct Config {
    endpoint: String,
    context_builder: ContextBuilder,
}

impl Config {
    pub(crate) fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            context_builder: empty_context_builder(),
        }
    }

    pub(crate) fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub(crate) fn set_endpoint(&mut self, endpoint: impl Into<String>) {
        self.endpoint = endpoint.into();
    }

    pub(crate) fn set_context_builder(&mut self, context_builder: ContextBuilder) {
        self.context_builder = context_builder;
    }

    pub(crate) async fn build_context(&self, request: ContextBuilderRequest) -> RequestContext {
        (self.context_builder)(request).await
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new("/rpc")
    }
}
