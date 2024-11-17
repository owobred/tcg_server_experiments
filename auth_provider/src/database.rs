pub mod models;

use futures::StreamExt;
use models::{DiscordOauthUser, DiscordUserId, Token, TokenHash, User, UserId};
use rand::{Rng, RngCore};
use sqlx::Executor;
use time::format_description::well_known::Iso8601;
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

    pub async fn create_auth_token(&self, user_id: &UserId) -> Result<Token, sqlx::Error> {
        let token_bytes = generate_random_token_bytes();

        let token = Token(token_bytes.clone());
        let token_hash = token.get_hash();

        sqlx::query("insert into auth_tokens(user_id, token_hash, expires_at) values (?, ?, ?)")
            .bind(user_id)
            .bind(&token_hash)
            .execute(&self.pool)
            .await?;

        Ok(token)
    }

    pub async fn get_user_id_by_token(&self, token: &Token) -> Result<Option<UserId>, sqlx::Error> {
        let result: Option<(UserId, String)> =
            sqlx::query_as("select user_id from auth_tokens where token_hash = ?")
                .bind(&token.get_hash())
                .fetch_optional(&self.pool)
                .await?;

        match result {
            Some((user_id, expires_at)) => {
                let expires_at_time = time::OffsetDateTime::parse(&expires_at, &Iso8601::DEFAULT)
                    .expect("stored timestamp was not valid?");

                if expires_at_time <= time::OffsetDateTime::now_utc() {
                    return Ok(None);
                }

                Ok(Some(user_id))
            }
            None => Ok(None),
        }
    }

    // NOTE: this wouldn't need to be anywhere near as complex if we used a real db like psql that can do datetimes natively
    pub async fn garbage_collect_expired_tokens(&self) -> Result<usize, sqlx::Error> {
        let mut num_deleted = 0;

        let now = time::OffsetDateTime::now_utc();

        let mut stream =
            sqlx::query_as("select (token_hash, expires_at) from auth_tokens").fetch(&self.pool);

        while let Some(entry) = stream.next().await {
            let (token_hash, expires_at): (Vec<u8>, String) = entry?;

            let expires_at_time = time::OffsetDateTime::parse(&expires_at, &Iso8601::DEFAULT)
                .expect("stored timestamp was not valid?");

            if expires_at_time <= now {
                num_deleted += 1;

                sqlx::query("delete from auth_tokens where token_hash = $1")
                    .bind(token_hash)
                    .execute(&self.pool)
                    .await?;
            }
        }

        Ok(num_deleted)
    }

    pub async fn revoke_auth_token(&self, token: &Token) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query("delete from auth_tokens where token_hash = $1")
            .bind(&token.get_hash())
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }
}

fn generate_random_token_bytes() -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut token_bytes = vec![0; 512];
    rng.fill_bytes(&mut token_bytes);
    token_bytes
}

fn generate_random_user_id(size: usize) -> UserId {
    let mut rng = rand::thread_rng();
    let mut user_id = String::with_capacity(size);

    for _ in 0..size {
        user_id.push(USER_ID_CHARACTERS[rng.gen_range(0..USER_ID_CHARACTERS.len())]);
    }

    UserId(user_id)
}
