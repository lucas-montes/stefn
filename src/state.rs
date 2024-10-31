use axum::extract::FromRef;
use jsonwebtoken::{DecodingKey, EncodingKey, Validation};

use super::config::ServiceConfig;

use crate::{
    auth::{create_validator, Keys},
    broker::Broker,
    config::SharedConfig,
    database::{Database, IpsDatabase},
};

#[derive(Clone)]
pub struct SharedState {
    database: Database,
    events_broker: Broker,
    ips_database: IpsDatabase,
}

impl SharedState {
    pub fn new(config: &SharedConfig) -> Self {
        Self {
            database: Database::new(&config.database_url),
            ips_database: IpsDatabase::new(&config.ips_database_url),
            events_broker: Broker::new(&config.broker_url),
        }
    }
    pub fn database(&self) -> &Database {
        &self.database
    }

    pub fn events_broker(&self) -> &Broker {
        &self.events_broker
    }
    pub fn ips_database(&self) -> &IpsDatabase {
        &self.ips_database
    }
}

#[derive(Clone, FromRef)]
pub struct WebsiteState {
    secrets: ServiceConfig,
    shared: SharedState,
    keys: Keys,
}

impl WebsiteState {
    pub fn new(secrets: ServiceConfig, shared: SharedState) -> Self {
        Self {
            keys: Keys::new(secrets.session_key.as_bytes()),
            shared,
            secrets,
        }
    }

    pub fn decoding(&self) -> &DecodingKey {
        &self.keys.decoding
    }
}

impl FromRef<WebsiteState> for Database {
    fn from_ref(app_state: &WebsiteState) -> Database {
        app_state.shared.database.clone()
    }
}
impl FromRef<WebsiteState> for Broker {
    fn from_ref(app_state: &WebsiteState) -> Broker {
        app_state.shared.events_broker.clone()
    }
}
impl FromRef<WebsiteState> for IpsDatabase {
    fn from_ref(app_state: &WebsiteState) -> IpsDatabase {
        app_state.shared.ips_database.clone()
    }
}

#[derive(Clone, FromRef)]
pub struct APIState {
    secrets: ServiceConfig,
    shared: SharedState,
    keys: Keys,
    jwt_validator: Validation,
}

impl APIState {
    pub fn new(secrets: ServiceConfig, shared: SharedState) -> Self {
        Self {
            keys: Keys::new(secrets.session_key.as_bytes()),
            jwt_validator: create_validator(secrets.domain()),
            shared,
            secrets,
        }
    }

    pub fn database(&self) -> &Database {
        &self.shared.database
    }

    pub fn encoding(&self) -> &EncodingKey {
        &self.keys.encoding
    }

    pub fn decoding(&self) -> &DecodingKey {
        &self.keys.decoding
    }

    pub fn validator(&self) -> &Validation {
        &self.jwt_validator
    }
}

impl FromRef<APIState> for Database {
    fn from_ref(app_state: &APIState) -> Database {
        app_state.shared.database.clone()
    }
}
impl FromRef<APIState> for Broker {
    fn from_ref(app_state: &APIState) -> Broker {
        app_state.shared.events_broker.clone()
    }
}
impl FromRef<APIState> for IpsDatabase {
    fn from_ref(app_state: &APIState) -> IpsDatabase {
        app_state.shared.ips_database.clone()
    }
}
