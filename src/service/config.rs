use axum::http::HeaderValue;
use menva::FromEnv;
use std::{net::Ipv4Addr, str::FromStr};

#[derive(Debug, Clone)]
pub enum Env {
    Development,
    Production,
    Test,
}

impl FromStr for Env {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" => Ok(Env::Development),
            "production" => Ok(Env::Production),
            "test" => Ok(Env::Test),
            _ => Err(format!("Invalid value for enum Env: {}", s)),
        }
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct SharedConfig {
    env: Env,
    max_upload_size: u64,
    ips_database_url: String,
    database_url: String,
    broker_url: String,
}

impl SharedConfig {
    pub fn stub() -> Self {
        Self {
            env: Env::Test,
            max_upload_size: 10485760,
            ips_database_url: "".into(),
            broker_url: "./test-broker.sqlite".to_owned(),
            database_url: "./test.sqlite".to_owned(),
        }
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ServiceConfig {
    ip: Ipv4Addr,
    port: u16,
    domain: String,
    allowed_origins: String,
    session_key: String,
}

impl ServiceConfig {
    pub fn stub() -> Self {
        Self {
            ip: Ipv4Addr::new(0, 0, 0, 0),
            port: 8000,
            domain: "test.com".into(),
            allowed_origins: "*".into(),
            session_key: "session_key".to_owned(),
        }
    }

    pub fn socket_addr(&self) -> (Ipv4Addr, u16) {
        (self.ip, self.port)
    }

    pub fn allowed_origins(&self) -> AllowedOrigins {
        AllowedOrigins::from_string(&self.allowed_origins)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    service: ServiceConfig,
    shared: SharedConfig,
}

impl Config {
    pub fn session_key(&self) -> &[u8] {
        &self.service.session_key.as_bytes()
    }
    pub fn database_url(&self) -> &str {
        &self.shared.database_url
    }
    pub fn ips_database_url(&self) -> &str {
        &self.shared.ips_database_url
    }
    pub fn broker_url(&self) -> &str {
        &self.shared.broker_url
    }
    pub fn domain(&self) -> &str {
        &self.service.domain
    }

    pub fn from_env(prefix: &str) -> Self {
        Self {
            service: ServiceConfig::from_env_with_prefix(prefix),
            shared: SharedConfig::from_env(),
        }
    }

    pub fn stub() -> Self {
        Self {
            service: ServiceConfig::stub(),
            shared: SharedConfig::stub(),
        }
    }

    pub fn socket_addr(&self) -> (Ipv4Addr, u16) {
        self.service.socket_addr()
    }

    pub fn allowed_origins(&self) -> AllowedOrigins {
        self.service.allowed_origins()
    }
}

#[derive(Clone, Debug, Default)]
pub struct AllowedOrigins(Vec<String>);

impl AllowedOrigins {
    fn from_string(allowed_origins: &str) -> Self {
        Self(allowed_origins.split(",").map(|s| s.to_owned()).collect())
    }

    pub fn to_headers(&self) -> Vec<HeaderValue> {
        self.0
            .iter()
            .filter_map(|s| s.parse::<HeaderValue>().ok())
            .collect()
    }
}
