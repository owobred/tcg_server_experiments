use std::sync::Arc;

use auth_provider::{
    database::{models::RefreshToken, Database},
    provider::discord::{self, DiscordInfo},
    WebState,
};
use axum::{extract::State, response::IntoResponse, Router};
use axum_extra::extract::CookieJar;
use tower_http::trace::TraceLayer;
use tracing::info;

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
        .with_state(web_state);

    let listener = tokio::net::TcpListener::bind("localhost:12121")
        .await
        .unwrap();

    info!(bind=%listener.local_addr().unwrap(), "created listener");

    axum::serve(listener, router).await.unwrap();
}

async fn auth_refresh(State(state): State<WebState>, jar: CookieJar) -> impl IntoResponse {
    let refresh = jar.get("RefreshToken");

    let refresh = match refresh {
        Some(cookie) => cookie.value(),
        None => return axum::Json(None),
    };

    let refresh = RefreshToken(refresh.to_owned());

    let session = state
        .database
        .create_session_token_from_refresh(&refresh)
        .await
        .unwrap();

    axum::Json(session)
}
