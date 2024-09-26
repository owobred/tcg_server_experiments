use axum::Router;

use crate::WebState;

pub mod discord;

pub fn route_all() -> Router<WebState> {
    Router::new().nest("/discord", discord::route())
}
