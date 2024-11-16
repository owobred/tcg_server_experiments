use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{OriginalUri, Query, State},
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use dashmap::DashSet;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    database::{
        models::{DiscordOauthUser, DiscordUserId, RefreshToken},
        Database,
    },
    WebState,
};

const STATE_CODE_CHARACTERS: [char; 52] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

const DISCORD_BASE: &'static str = "https://discord.com";

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct StateCode(String);

impl StateCode {
    pub fn get(&self) -> &str {
        &self.0
    }
}

#[derive(Clone)]
pub struct Authenticator {
    database: Arc<Database>,
    state_codes: Arc<DashSet<StateCode>>,
    client: reqwest::Client,
    info: Arc<DiscordInfo>,
}

impl Authenticator {
    pub fn new(database: Arc<Database>, info: DiscordInfo) -> Self {
        Self {
            database,
            state_codes: Arc::default(),
            client: reqwest::ClientBuilder::new()
                .https_only(true)
                .user_agent(concat!(
                    "DiscordBot (github.com/NeuroTCG/backend, ",
                    env!("CARGO_PKG_VERSION"),
                    ")"
                ))
                .build()
                .unwrap(),
            info: Arc::new(info),
        }
    }

    pub fn start_auth(&self) -> StateCode {
        let state_code = loop {
            let code = self.generate_state_code(32);
            if self.state_codes.contains(&code) {
                continue;
            } else {
                break code;
            }
        };

        self.state_codes.insert(state_code.clone());

        state_code
    }

    pub async fn auth_response(
        &self,
        state_code: &str,
        redirect_code: &str,
        redirect_uri: &str,
    ) -> RefreshToken {
        let state_code = StateCode(state_code.to_string());

        if !self.state_codes.contains(&state_code) {
            todo!()
        }

        // FIXME: this should not panic if discord returns an invalid response (i.e. we sent a bad request, user provided a bad auth code)
        let discord_token_info: DiscordTokenResponse = self
            .client
            .post(format!("{DISCORD_BASE}/api/v10/oauth2/token"))
            .form(&HashMap::from([
                ("grant_type", "authorization_code"),
                ("code", redirect_code),
                ("redirect_uri", redirect_uri),
            ]))
            .basic_auth(
                self.info.client_id.to_owned(),
                Some(self.info.client_secret.to_owned()),
            )
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let discord_auth_info: DiscordAuthInfoResponse = self
            .client
            .get(format!("{DISCORD_BASE}/api/v10/oauth2/@me"))
            .bearer_auth(&discord_token_info.access_token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let user_id = if self
            .database
            .discord_user_registered(&discord_auth_info.user.id)
            .await
            .unwrap()
        {
            self.database
                .get_user_by_discord_id(&discord_auth_info.user.id)
                .await
                .unwrap()
                .unwrap()
        } else {
            let user = self.database.create_new_user().await.unwrap();
            self.database
                .link_discord_id_to_user_id(
                    &user.user_id,
                    &DiscordOauthUser {
                        discord_id: discord_auth_info.user.id,
                        linked_to_user_id: user.user_id.clone(),
                        refresh_token: discord_token_info.refresh_token,
                        access_token: discord_token_info.access_token,
                        expires_at: discord_auth_info.expires,
                    },
                )
                .await
                .unwrap();

            user.user_id
        };

        let auth_token = self
            .database
            .create_auth_refresh_token(&user_id)
            .await
            .unwrap();

        auth_token
    }

    fn generate_state_code(&self, size: usize) -> StateCode {
        let mut rng = rand::thread_rng();

        let mut random_string = String::with_capacity(size);

        for _ in 0..size {
            random_string
                .push(STATE_CODE_CHARACTERS[rng.gen_range(0..STATE_CODE_CHARACTERS.len())]);
        }

        let state_code = StateCode(random_string);

        return state_code;
    }
}

pub struct DiscordInfo {
    client_id: String,
    client_secret: String,
}

impl DiscordInfo {
    pub fn from_env() -> Self {
        Self {
            client_id: std::env::var("DISCORD_OAUTH_CLIENT_ID")
                .expect("failed to read DISCORD_OAUTH_CLIENT_ID environment variable"),
            client_secret: std::env::var("DISCORD_OAUTH_CLIENT_SECRET")
                .expect("failed to read DISCORD_OAUTH_CLIENT_SECRET environment variable"),
        }
    }
}

async fn start_auth(State(state): State<WebState>) -> impl IntoResponse {
    let state_code = state.discord_authenticator.start_auth();

    // TODO: figure out how to make this use the DISCORD_BASE constant?
    let uri = http::Uri::builder()
        .scheme("https")
        .authority("discord.com")
        .path_and_query(format!(
            "/oauth2/authorize?{}",
            url::form_urlencoded::Serializer::new(String::new())
                .append_pair("client_id", &state.discord_authenticator.info.client_id)
                .append_pair("response_type", "code")
                .append_pair(
                    "redirect_uri",
                    &format!("{}/auth/providers/discord/redirect", state.webserver_base)
                )
                .append_pair("scope", "identify")
                .append_pair("state", state_code.get())
                .finish()
        ))
        .build()
        .unwrap();

    Redirect::to(&uri.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct QueryParams {
    state: String,
    code: String,
}

async fn handle_redirect(
    State(state): State<WebState>,
    Query(params): Query<QueryParams>,
    OriginalUri(uri): OriginalUri,
) -> impl IntoResponse {
    let refresh_token = state
        .discord_authenticator
        .auth_response(
            &params.state,
            &params.code,
            &format!("{}{}", state.webserver_base, uri.path()),
        )
        .await;

    let mut response = Redirect::to("/").into_response();

    response
        .headers_mut()
        // TODO: add an `Expires` to the refresh token?
        //       also maybe set `Path` to `/auth/refresh`
        .append(
            "Set-Cookie",
            format!(
                "RefreshToken={}; HttpOnly; SameSite=Strict; Secure; Path=/auth/refresh",
                refresh_token.0
            )
            .try_into()
            .unwrap(),
        );

    response
}

pub fn routes() -> Router<WebState> {
    Router::new()
        .route("/begin", get(start_auth))
        .route("/redirect", get(handle_redirect))
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: isize,
    refresh_token: String,
    scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscordAuthInfoResponse {
    application: DiscordApplicationResponse,
    scopes: Vec<String>,
    #[serde(with = "time::serde::iso8601")]
    expires: time::OffsetDateTime,
    user: DiscordUserResponse,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscordApplicationResponse {
    id: String,
    // other fields are not relevant
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscordUserResponse {
    id: DiscordUserId,
    username: String,
    global_name: String,
    avatar: String,
    discriminator: String,
    public_flags: usize,
}
