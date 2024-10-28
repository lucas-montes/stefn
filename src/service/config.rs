use axum::http::HeaderValue;
use menva::FromEnv;
use serde::{Deserialize, Serialize};
use std::{fs::File, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone, FromEnv)]
pub struct Config {
    pub env: Env,
    ip: String,
    port: u16,
    pub max_upload_size: u64,
    pub domain: String,
    allowed_origins: String,
    pub ips_database_url: String,
    pub database_url: String,
    pub broker_url: String,
    pub session_key: String,
}

impl Config {
    pub fn from_file(file_name: &str) -> Self {
        //TODO: would be cool to read from an encrypted file
        serde_json::from_reader(File::open(file_name).expect("where is your config file?"))
            .expect("your config is wrong")
    }

    pub fn stub() -> Self {
        Self {
            env: Env::Test,
            ip: "127.0.0.1".into(),
            port: 8000,
            max_upload_size: 10485760,
            domain: "test.com".into(),
            allowed_origins: "*".into(),
            ips_database_url: "".into(),
            broker_url: "./test-broker.sqlite".to_owned(),
            database_url: "./test.sqlite".to_owned(),
            session_key: "session_key".to_owned(),
        }
    }

    pub fn socket_addr(&self) -> (&str, u16) {
        (&self.ip, self.port)
    }

    pub fn allowed_origins(&self) -> AllowedOrigins {
        AllowedOrigins::from_string(&self.allowed_origins)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
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
