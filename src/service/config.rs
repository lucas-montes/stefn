use axum::http::HeaderValue;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    net::{IpAddr, Ipv4Addr},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Env {
    Development,
    Production,
    Test,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceConfig {
    pub env: Env,
    pub ip: IpAddr,
    pub port: u16,
    pub max_upload_size: u64,
    pub domain_name: String,
    pub allowed_origins: AllowedOrigins,
    pub ips_database: Option<String>,
    pub database_url: String,
    pub session_key: String,
}

impl ServiceConfig {
    pub fn from_file(file_name: &str) -> Self {
        serde_json::from_reader(File::open(file_name).expect("where is your config file?"))
            .expect("your config is wrong")
    }

    pub fn stub() -> Self {
        Self {
            env: Env::Test,
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 8000,
            max_upload_size: 10485760,
            domain_name: "test.com".to_owned(),
            allowed_origins: AllowedOrigins::default(),
            ips_database: None,
            database_url: "./test.sqlite".to_owned(),
            session_key: "session_key".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AllowedOrigins(Vec<String>);

impl AllowedOrigins {
    fn default() -> Self {
        Self(vec!["*".to_owned()])
    }

    pub fn to_headers(&self) -> Vec<HeaderValue> {
        self.0
            .iter()
            .filter_map(|s| s.parse::<HeaderValue>().ok())
            .collect()
    }
}
