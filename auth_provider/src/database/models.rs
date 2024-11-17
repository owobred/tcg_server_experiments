use blake2::Digest;
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

#[derive(Debug, Clone, Type)]
#[sqlx(transparent)]
pub struct Token(pub Vec<u8>);

impl Token {
    pub fn get_hash(&self) -> TokenHash {
        let mut hasher = blake2::Blake2b512::new();
        hasher.update(&self.0);

        let result = hasher.finalize();

        TokenHash(result[..].to_vec())
    }

    pub fn to_hex_string(&self) -> String {
        hex::encode(&self.0)
    }

    pub fn from_hex_string(hex_string: &str) -> Option<Self> {
        hex::decode(hex_string).map(|bytes| Token(bytes)).ok()
    }
}

#[derive(Debug, Clone, Type)]
#[sqlx(transparent)]
pub struct TokenHash(pub Vec<u8>);
