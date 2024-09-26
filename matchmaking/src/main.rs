use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
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
                            .with_env_var("MATCHMAKER_LOG")
                            .from_env()
                            .expect("failed to parse env"),
                    ),
            )
            .init();
    }

    let router = Router::new()
        .layer(TraceLayer::new_for_http())
        .route("/matchmake", get(get_matchmake))
        .route("/servers", post(post_servers))
        .with_state(MatchmakerState);

    let listener = tokio::net::TcpListener::bind("localhost:12121")
        .await
        .unwrap();

    info!(bind=%listener.local_addr().unwrap(), "created listener");

    axum::serve(listener, router).await.unwrap();
}

#[derive(Debug, Clone)]
struct MatchmakerState;

async fn get_matchmake() -> impl IntoResponse {
    ()
}

async fn post_servers() -> impl IntoResponse {
    ()
}

// async fn websocket_handle() {
//     let user_info = authenticate().await?;
//
//     loop {
//         send_available_matchmaking_options().await?;
//
//         let matchmaking_options = get_matchmaking_options().await;
//
//         let (can_matchmake, reason) =
//             are_matchmaking_options_available(matchmaking_options, user_info);
//
//         if !can_matchmake {
//             close_connection(reason).await;
//             break;
//         }
//
//         let result = tokio::select! {
//             game = matchmake(matchmaking_options, user_info) => MmResult::Match(game),
//             reason = get_cancel_message() => MmResult::ClientCancelled(reason),
//             reason = get_server_cancel_message() => MmResult::ServerCancelled(reason),
//         };
//
//         match result {
//             MmResult::Match(match_info) => {
//                 send_match_info(match_info).await;
//
//                 // create task here instead / add timeout before closing?
//                 close_connection("ok").await;
//                 break;
//             }
//             MmResult::ClientCancelled(reason) => match reason {
//                 Reason::Disconnect => break,
//                 Reason::UserCancelled => continue,
//             },
//             MmResult::ServerCancelled(reason) => match reason {
//                 Reason::ServerShuttingDown => {
//                     close_connection("server_shutting_down").await;
//                     break;
//                 }
//             },
//         }
//     }
// }
