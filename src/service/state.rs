use super::config::Config;
use axum::extract::{FromRequestParts, State};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::{ops::Deref, str::FromStr, sync::Arc};

use crate::{auth::Keys, broker::Broker};

pub struct App {
    pub primary_database: SqlitePool,
    pub events_broker: Broker,
    pub ips_database: Option<maxminddb::Reader<Vec<u8>>>,
    pub keys: Keys,
    pub config: Config,
}

impl App {
    pub fn new(config: Config) -> Self {
        let ips_database = if config.ips_database_url.is_empty() {
            None
        } else {
            Some(
                maxminddb::Reader::open_readfile(&config.ips_database_url)
                    .expect("the database for the ips seems to be missing or is the wrong path"),
            )
        };

        let database_config = SqliteConnectOptions::from_str(&config.database_url)
            .expect("Cannot connect to database")
            .create_if_missing(true);
        Self {
            keys: Keys::new(config.session_key.as_bytes()),
            primary_database: SqlitePool::connect_lazy_with(database_config),
            ips_database,
            events_broker: Broker::new(&config.broker_url),
            config,
        }
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
