use crate::event::EventRequest;

pub trait Guard: Default + Send + Sync + 'static {
    fn can_proceed(&self, context: &GuardContext) -> bool;
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuardContext {
    headers: RequestHeaders,
    request: EventRequest,
}

impl GuardContext {
    pub(crate) fn new(headers: RequestHeaders, request: EventRequest) -> Self {
        Self { headers, request }
    }

    pub fn headers(&self) -> &RequestHeaders {
        &self.headers
    }

    pub fn request(&self) -> &EventRequest {
        &self.request
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RequestHeaders {
    headers: Vec<RequestHeader>,
}

impl RequestHeaders {
    pub fn new<N, V>(headers: impl IntoIterator<Item = (N, V)>) -> Self
    where
        N: Into<String>,
        V: Into<String>,
    {
        Self {
            headers: headers
                .into_iter()
                .map(|(name, value)| RequestHeader {
                    name: name.into(),
                    value: value.into(),
                })
                .collect(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.values(name).next()
    }

    pub fn values(&self, name: &str) -> impl Iterator<Item = &str> {
        let name = name.to_owned();
        self.headers
            .iter()
            .filter(move |header| header.name.eq_ignore_ascii_case(&name))
            .map(|header| header.value.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers
            .iter()
            .map(|header| (header.name.as_str(), header.value.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestHeader {
    name: String,
    value: String,
}

impl RequestHeader {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}
