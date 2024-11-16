use axum::Router;

use crate::WebState;

pub mod discord;

pub fn all_routes() -> Router<WebState> {
    Router::new().nest("/discord", discord::routes())
}
