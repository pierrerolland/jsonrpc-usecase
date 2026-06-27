#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    endpoint: String,
}

impl Config {
    pub(crate) fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    pub(crate) fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new("/rpc")
    }
}
