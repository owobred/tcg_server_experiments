use std::sync::Arc;

use database::Database;
use provider::discord;

pub mod database;
pub mod provider;

#[derive(Clone)]
pub struct WebState {
    pub database: Arc<Database>,
    pub webserver_base: Arc<String>,
    pub discord_authenticator: Arc<discord::Authenticator>,
}
