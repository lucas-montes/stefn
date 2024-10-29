use super::config::Config;
use axum::extract::{FromRequestParts, State};
use std::{ops::Deref, sync::Arc};

use crate::{
    auth::Keys,
    broker::Broker,
    database::{Database, IpsDatabase},
};

pub struct App {
    pub primary_database: Database,
    pub events_broker: Broker,
    pub ips_database: IpsDatabase,
    pub keys: Keys,
    pub config: Config,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            keys: Keys::new(config.session_key()),
            primary_database: Database::new(&config.database_url()),
            ips_database: IpsDatabase::new(&config.ips_database_url()),
            events_broker: Broker::new(&config.broker_url()),
            config,
        }
    }

    pub fn domain(&self) -> &str {
        &self.config.domain()
    }
}

#[derive(Clone, FromRequestParts)]
#[from_request(via(State))]
pub struct AppState(pub Arc<App>);

impl AppState {
    pub fn new(config: Config) -> Self {
        AppState(Arc::new(App::new(config)))
    }
}

impl Deref for AppState {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
