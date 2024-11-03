use axum::extract::FromRef;
use jsonwebtoken::{DecodingKey, EncodingKey, Validation};

use crate::{
    auth::{create_validator, Keys},
    broker::Broker,
    config::{APIConfig, ServiceConfig, SharedConfig, WebsiteConfig},
    database::{Database, IpsDatabase},
    sessions::Sessions,
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
    secrets: WebsiteConfig,
    shared: SharedState,
    sessions: Sessions,
}

impl WebsiteState {
    pub fn new(secrets: WebsiteConfig, shared: SharedState) -> Self {
        Self {
            sessions: Sessions::new(&secrets.sessions_db),
            shared,
            secrets,
        }
    }

    pub fn events_broker(&self) -> &Broker {
        &self.shared.events_broker
    }
    pub fn ips_database(&self) -> &IpsDatabase {
        &self.shared.ips_database
    }

    pub fn config(&self) -> &WebsiteConfig {
        &self.secrets
    }

    pub fn sessions(&self) -> &Sessions {
        &self.sessions
    }

    pub fn database(&self) -> &Database {
        &self.shared.database
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

#[derive(Clone, FromRef)]
pub struct APIState {
    secrets: APIConfig,
    shared: SharedState,
    keys: Keys,
    jwt_validator: Validation,
}

impl APIState {
    pub fn new(secrets: APIConfig, shared: SharedState) -> Self {
        Self {
            keys: Keys::new(secrets.session_key.as_bytes()),
            jwt_validator: create_validator(secrets.domain()),
            shared,
            secrets,
        }
    }

    pub fn events_broker(&self) -> &Broker {
        &self.shared.events_broker
    }
    pub fn ips_database(&self) -> &IpsDatabase {
        &self.shared.ips_database
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
