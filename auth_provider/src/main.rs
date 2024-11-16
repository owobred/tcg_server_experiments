use std::sync::Arc;

use auth_provider::{
    database::{models::RefreshToken, Database, SessionToken},
    provider::discord::{self, DiscordInfo},
    WebState,
};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Router,
};
use axum_extra::extract::CookieJar;
use tower_http::trace::TraceLayer;
use tracing::{info, trace};

#[tokio::main]
async fn main() {
    {
        use tracing_subscriber::prelude::*;

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_filter(
                        tracing_subscriber::EnvFilter::builder()
                            .with_default_directive(
                                tracing::level_filters::LevelFilter::INFO.into(),
                            )
                            .with_env_var("AUTH_LOG")
                            .from_env()
                            .expect("failed to parse env"),
                    ),
            )
            .init();
    }

    let webserver_base =
        std::env::var("DOMAIN_BASE").expect("did not find DOMAIN_BASE environment variable");

    let database = Arc::new(
        Database::new("./data.db")
            .await
            .expect("failed to open database"),
    );

    let web_state = WebState {
        database: database.clone(),
        webserver_base: Arc::new(webserver_base),
        discord_authenticator: Arc::new(discord::Authenticator::new(
            database,
            DiscordInfo::from_env(),
        )),
    };

    let router = Router::new()
        .layer(TraceLayer::new_for_http())
        .nest("/auth/providers", auth_provider::provider::all_routes())
        .route("/auth/refresh", axum::routing::get(auth_refresh))
        .route("/auth/logout", axum::routing::post(auth_invalidate))
        .with_state(web_state);

    let listener = tokio::net::TcpListener::bind("localhost:12121")
        .await
        .unwrap();

    info!(bind=%listener.local_addr().unwrap(), "created listener");

    axum::serve(listener, router).await.unwrap();
}

async fn auth_refresh(State(state): State<WebState>, jar: CookieJar) -> AuthRefreshResponse {
    let refresh = jar.get("RefreshToken");

    let refresh = match refresh {
        Some(cookie) => cookie.value(),
        None => return AuthRefreshResponse::NoCookie,
    };

    let refresh = RefreshToken(refresh.to_owned());

    let session = state
        .database
        .create_session_token_from_refresh(&refresh)
        .await
        .unwrap();

    match session {
        Some(session) => AuthRefreshResponse::ValidCookie(session),
        None => AuthRefreshResponse::InvalidCookie(jar),
    }
}

pub enum AuthRefreshResponse {
    NoCookie,
    InvalidCookie(CookieJar),
    ValidCookie(SessionToken),
}

impl IntoResponse for AuthRefreshResponse {
    fn into_response(self) -> Response {
        match self {
            AuthRefreshResponse::NoCookie => Response::builder()
                .status(axum::http::StatusCode::BAD_REQUEST)
                .body("missing RefreshToken cookie".into())
                .unwrap(),
            AuthRefreshResponse::InvalidCookie(cookie_jar) => (
                cookie_jar.remove("RefreshToken"),
                axum::http::StatusCode::BAD_REQUEST,
            )
                .into_response(),
            AuthRefreshResponse::ValidCookie(session_token) => Response::builder()
                .status(axum::http::StatusCode::OK)
                .body(session_token.0.into())
                .unwrap(),
        }
    }
}

async fn auth_invalidate(State(state): State<WebState>, jar: CookieJar) -> impl IntoResponse {
    let refresh = jar.get("RefreshToken");

    let refresh_token = match refresh {
        Some(token) => token.value(),
        None => return jar,
    };

    let was_real_token = state
        .database
        .revoke_auth_token(&RefreshToken(refresh_token.to_owned()))
        .await
        .unwrap();

    trace!(?was_real_token, "revoked token");

    jar.remove("RefreshToken")
}
