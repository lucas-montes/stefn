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
    pub ips_database_url: String,
    pub database_url: String,
    pub broker_url: String,
    pub worker_threads: usize,
    pub max_blocking_threads: usize,
}

impl SharedConfig {
    pub fn stub() -> Self {
        Self {
            env: Env::Test,
            max_upload_size: 10485760,
            ips_database_url: "".into(),
            broker_url: "./test-broker.sqlite".to_owned(),
            database_url: "./test.sqlite".to_owned(),
            worker_threads: 1,
            max_blocking_threads: 1,
        }
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ServiceConfig {
    ip: Ipv4Addr,
    port: u16,
    domain: String,
    allowed_origins: String,
    pub session_key: String,
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

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn print(&self) {
        println!("http://{:?}:{:?}", &self.ip, &self.port)
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
