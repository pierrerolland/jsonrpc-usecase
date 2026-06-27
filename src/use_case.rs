use crate::Error;
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;

pub trait UseCaseDefinition: Send + Sync + 'static {
    type Input: DeserializeOwned + Send + 'static;
    type Output: Serialize + Send + 'static;
    type Error: Error;

    fn execute(
        &self,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}
