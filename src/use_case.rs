use crate::{
    Error, GuardContext, InputValidationErrors, UseCaseExecutionError,
    input::process_registered_input,
};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;

pub trait UseCaseDefinition: Send + Sync + 'static {
    type Input: DeserializeOwned + Send + Sync + 'static;
    type Output: Serialize + Send + Sync + 'static;
    type Error: Error;

    const WILL_EVENT: &'static str;
    const DID_EVENT: &'static str;

    fn can_proceed(context: &GuardContext) -> bool;

    fn prepare_input(mut input: Self::Input) -> Result<Self::Input, InputValidationErrors> {
        process_registered_input(&mut input)?;
        Ok(input)
    }

    fn execute_prepared(
        &self,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;

    fn execute(
        &self,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, UseCaseExecutionError<Self::Error>>> + Send {
        async move {
            let input = Self::prepare_input(input)?;
            self.execute_prepared(input)
                .await
                .map_err(UseCaseExecutionError::Execution)
        }
    }
}
