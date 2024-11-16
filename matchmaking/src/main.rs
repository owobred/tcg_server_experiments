use axum::Router;
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

    let router = Router::new().layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("localhost:12121")
        .await
        .unwrap();

    info!(bind=%listener.local_addr().unwrap(), "created listener");

    axum::serve(listener, router).await.unwrap();
}
