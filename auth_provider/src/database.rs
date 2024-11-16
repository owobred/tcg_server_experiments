pub mod models;

use futures::StreamExt;
use models::{DiscordOauthUser, DiscordUserId, RefreshToken, User, UserId};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::Executor;
use tracing::warn;

const USER_ID_CHARACTERS: [char; 62] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];

pub struct Database {
    pool: sqlx::SqlitePool,
}

impl Database {
    pub async fn new(file: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::SqlitePool::connect(file).await?;

        let mut results = pool.execute_many(include_str!("../schema.sql"));

        while let Some(result) = results.next().await {
            match result {
                Ok(_) => (),
                Err(error) => warn!(?error, "error when creating initial database tables"),
            }
        }

        Ok(Self { pool })
    }

    pub async fn user_id_exists(&self, user_id: &UserId) -> Result<bool, sqlx::Error> {
        let row: Option<_> = sqlx::query("select 1 from users where user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.is_some())
    }

    pub async fn create_new_user(&self) -> Result<User, sqlx::Error> {
        let user_id = loop {
            let user_id = generate_random_user_id(24);

            if self.user_id_exists(&user_id).await? {
                continue;
            }

            break user_id;
        };

        sqlx::query("insert into users (user_id) values (?)")
            .bind(&user_id)
            .execute(&self.pool)
            .await?;

        Ok(User { user_id })
    }

    pub async fn discord_user_registered(
        &self,
        user_id: &DiscordUserId,
    ) -> Result<bool, sqlx::Error> {
        Ok(
            sqlx::query("select 1 from discord_oauth_users where discord_id = ?")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?
                .is_some(),
        )
    }

    pub async fn link_discord_id_to_user_id(
        &self,
        user_id: &UserId,
        discord_info: &DiscordOauthUser,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "insert into discord_oauth_users (
            linked_to_user_id,
            discord_id,
            refresh_token,
            access_token,
            expires_at
        ) values (?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(&discord_info.discord_id)
        .bind(&discord_info.refresh_token)
        .bind(&discord_info.refresh_token)
        .bind(&discord_info.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_by_discord_id(
        &self,
        user_id: &DiscordUserId,
    ) -> Result<Option<UserId>, sqlx::Error> {
        sqlx::query_as("select linked_to_user_id from discord_oauth_users where discord_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_auth_refresh_token(
        &self,
        user_id: &UserId,
    ) -> Result<RefreshToken, sqlx::Error> {
        // FIXME: do this properly smh
        let token = RefreshToken(generate_random_user_id(128).0);
        sqlx::query("insert into auth_refresh_tokens(user_id, token_hash) values (?, ?)")
            .bind(user_id)
            .bind(&token)
            .execute(&self.pool)
            .await?;
        Ok(token)
    }

    pub async fn get_user_by_refresh_token(
        &self,
        refresh_token: &RefreshToken,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as("select user_id from auth_refresh_tokens where token_hash = ?")
            .bind(refresh_token)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_session_token_from_refresh(
        &self,
        refresh_token: &RefreshToken,
    ) -> Result<Option<SessionToken>, sqlx::Error> {
        let user = self.get_user_by_refresh_token(refresh_token).await?;
        let user = match user {
            Some(user) => user,
            None => return Ok(None),
        };

        // TODO: create a proper jwt here or something

        Ok(Some(SessionToken(user.user_id.0)))
    }

    pub async fn revoke_auth_token(
        &self,
        refresh_token: &RefreshToken,
    ) -> Result<bool, sqlx::Error> {
        Ok(
            sqlx::query("delete from auth_refresh_tokens where token_hash = $1")
                .bind(&refresh_token)
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }
}

fn generate_random_user_id(size: usize) -> UserId {
    let mut rng = rand::thread_rng();
    let mut user_id = String::with_capacity(size);

    for _ in 0..size {
        user_id.push(USER_ID_CHARACTERS[rng.gen_range(0..USER_ID_CHARACTERS.len())]);
    }

    UserId(user_id)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionToken(pub String);
