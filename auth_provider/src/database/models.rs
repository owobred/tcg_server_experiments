use serde::{Deserialize, Serialize};
use sqlx::prelude::{FromRow, Type};

#[derive(Debug, Clone, FromRow)]
pub struct DiscordOauthUser {
    pub discord_id: DiscordUserId,
    pub linked_to_user_id: UserId,
    pub refresh_token: String,
    pub access_token: String,
    pub expires_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct DiscordUserId(pub String);

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub user_id: UserId,
}

#[derive(Debug, Clone, Type, FromRow)]
#[sqlx(transparent)]
pub struct UserId(pub String);

#[derive(Debug, Clone, Type, Serialize, Deserialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct RefreshToken(pub String);