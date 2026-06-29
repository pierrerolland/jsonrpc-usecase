use crate::{Error, GuardContext};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;

pub trait UseCaseDefinition: Send + Sync + 'static {
    type Input: DeserializeOwned + Send + Sync + 'static;
    type Output: Serialize + Send + Sync + 'static;
    type Error: Error;

    const WILL_EVENT: &'static str;
    const DID_EVENT: &'static str;

    fn can_proceed(context: &GuardContext) -> bool;

    fn execute(
        &self,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}
