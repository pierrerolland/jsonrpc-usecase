use crate::guard::RequestHeaders;
use std::{
    any::Any,
    cell::RefCell,
    fmt::{self, Debug, Formatter},
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context as TaskContext, Poll},
};

type TypedContext = Arc<dyn Any + Send + Sync>;

thread_local! {
    static CURRENT_CONTEXTS: RefCell<Vec<RequestContext>> = const { RefCell::new(Vec::new()) };
}

pub(crate) type ContextBuilder =
    Arc<dyn Fn(ContextBuilderRequest) -> ContextBuilderFuture + Send + Sync>;
pub(crate) type ContextBuilderFuture = Pin<Box<dyn Future<Output = RequestContext> + Send>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextBuilderRequest {
    headers: RequestHeaders,
}

impl ContextBuilderRequest {
    pub(crate) fn new(headers: RequestHeaders) -> Self {
        Self { headers }
    }

    pub fn headers(&self) -> &RequestHeaders {
        &self.headers
    }
}

#[derive(Clone, Default)]
pub struct RequestContext {
    payload: Option<TypedContext>,
}

impl RequestContext {
    pub fn empty() -> Self {
        Self { payload: None }
    }

    pub fn new<T>(context: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self {
            payload: Some(Arc::new(context)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.payload.is_none()
    }

    pub fn get<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.payload.as_ref()?.as_ref().downcast_ref()
    }

    pub fn get_cloned<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.payload.as_ref()?.clone().downcast().ok()
    }
}

impl Debug for RequestContext {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RequestContext")
            .field("is_empty", &self.is_empty())
            .finish_non_exhaustive()
    }
}

impl PartialEq for RequestContext {
    fn eq(&self, other: &Self) -> bool {
        match (&self.payload, &other.payload) {
            (None, None) => true,
            (Some(left), Some(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }
}

pub fn current_context<T>() -> Option<Arc<T>>
where
    T: Send + Sync + 'static,
{
    CURRENT_CONTEXTS.with(|contexts| contexts.borrow().last()?.get_cloned())
}

pub fn with_current_context<T, Output>(f: impl FnOnce(&T) -> Output) -> Option<Output>
where
    T: 'static,
{
    CURRENT_CONTEXTS.with(|contexts| contexts.borrow().last()?.get().map(f))
}

pub(crate) fn empty_context_builder() -> ContextBuilder {
    Arc::new(|_| Box::pin(async { RequestContext::empty() }))
}

pub(crate) fn scope<F>(context: RequestContext, future: F) -> ContextScope<F>
where
    F: Future,
{
    ContextScope {
        context,
        future: Box::pin(future),
    }
}

pub(crate) struct ContextScope<F>
where
    F: Future,
{
    context: RequestContext,
    future: Pin<Box<F>>,
}

impl<F> Future for ContextScope<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, context: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let _guard = ContextStackGuard::new(self.context.clone());

        self.future.as_mut().poll(context)
    }
}

struct ContextStackGuard;

impl ContextStackGuard {
    fn new(context: RequestContext) -> Self {
        CURRENT_CONTEXTS.with(|contexts| contexts.borrow_mut().push(context));

        Self
    }
}

impl Drop for ContextStackGuard {
    fn drop(&mut self) {
        CURRENT_CONTEXTS.with(|contexts| {
            contexts
                .borrow_mut()
                .pop()
                .expect("request context stack should not be empty");
        });
    }
}
