use thiserror::Error;

use super::AuthenticationProvider;

#[derive(Debug, Error)]
#[error("this error only exists because the `!` type is not stable yet")]
pub struct DummyError;

pub struct DummyAuthenticationProvider<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> AuthenticationProvider for DummyAuthenticationProvider<T>
where
    T: Clone + Send + Sync,
{
    type AuthenticationToken = T;
    type UserId = T;
    type Error = DummyError;

    async fn authenticate(
        &self,
        token: &Self::AuthenticationToken,
    ) -> Result<Option<Self::UserId>, Self::Error> {
        Ok(Some(token.to_owned()))
    }
}
