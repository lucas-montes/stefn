use super::config::ServiceConfig;
use axum::extract::{FromRequestParts, State};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::{ops::Deref, sync::Arc};

use crate::{broker::Broker, service::Keys};

pub struct App {
    pub primary_database: SqlitePool,
    pub events_broker: Broker,
    pub ips_database: Option<maxminddb::Reader<Vec<u8>>>,
    pub keys: Keys,
    pub domain: String,
}

impl App {
    pub fn new(config: &ServiceConfig) -> Self {
        let ips_database = config.ips_database.as_ref().map(|f| {
            maxminddb::Reader::open_readfile(f)
                .expect("the database for the ips seems to be missing or is the wrong path")
        });

        let database_config = SqliteConnectOptions::new()
            .filename(&config.database_url)
            .create_if_missing(true);
        Self {
            keys: Keys::new(config.session_key.as_bytes()),
            primary_database: SqlitePool::connect_lazy_with(database_config),
            ips_database,
            events_broker: Broker::new(&config.broker_url),
            domain: config.domain_name.clone(),
        }
    }
}

#[derive(Clone, FromRequestParts)]
#[from_request(via(State))]
pub struct AppState(pub Arc<App>);

impl AppState {
    pub fn new(config: &ServiceConfig) -> Self {
        AppState(Arc::new(App::new(&config)))
    }
}

impl Deref for AppState {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
