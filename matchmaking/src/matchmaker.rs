use std::sync::Arc;

use crate::auth::AuthenticationProvider;

pub struct Matchmaker<A, Token>
where
    A: AuthenticationProvider<AuthenticationToken = Token>,
{
    authentication_provider: Arc<A>,
}

impl<A, Token> Matchmaker<A, Token>
where
    A: AuthenticationProvider<AuthenticationToken = Token>,
{
    pub fn create_user_handler(&self) -> UserHandler<A, Token> {
        UserHandler {
            authentication_provider: self.authentication_provider.clone(),
        }
    }
}

pub struct UserHandler<A, Token>
where
    A: AuthenticationProvider<AuthenticationToken = Token>,
{
    authentication_provider: Arc<A>,
}

impl<A, Token> UserHandler<A, Token>
where
    A: AuthenticationProvider<AuthenticationToken = Token>,
{
    async fn handle_user_connection<C, M>(self, mut connection: C)
    where
        C: RawUserConnection<RawMessage = M>,
        M: TryCastMatchmakingMessage<AuthenticationToken = Token>,
    {
        let user_id = connection.next_message().await;
    }
}

pub trait RawUserConnection {
    type RawMessage;

    fn next_message(&mut self) -> impl std::future::Future<Output = Option<Self::RawMessage>>;
}

pub trait TryCastMatchmakingMessage
where
    Self: Sized,
{
    type AuthenticationToken;
    type MatchmakingCancel;

    fn try_as_auth_token(self) -> Result<Self::AuthenticationToken, Self>;
    fn try_as_matchmaking_cancel(self) -> Result<Self::MatchmakingCancel, Self>;
    fn try_as
}
