pub mod providers;

pub trait AuthenticationProvider {
    type AuthenticationToken;
    type UserId;
    type Error: std::error::Error;

    fn authenticate(
        &self,
        token: &Self::AuthenticationToken,
    ) -> impl std::future::Future<Output = Result<Option<Self::UserId>, Self::Error>> + Send;
}