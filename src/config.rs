use axum::http::HeaderValue;
use menva::FromEnv;
use oauth2::Scope;
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
    pub env: Env,
    pub max_upload_size: u64,
    pub ips_database_url: String,
    pub database_url: String,
    pub broker_url: String,
    pub worker_threads: usize,
    pub max_blocking_threads: usize,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_relay: String,
    pub stripe_private_key: String,
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
            smtp_username: "smtp_username".to_owned(),
            smtp_password: "smtp_password".to_owned(),
            smtp_relay: "smtp_relay".to_owned(),
            stripe_private_key: "stripe_private_key".into(),
        }
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct WebsiteConfig {
    ip: Ipv4Addr,
    port: u16,
    domain: String,
    allowed_origins: String,
    pub session_key: String,
    pub sessions_db: String,
    pub session_cookie_name: String,
    pub session_expiration: i64,
    pub login_redirect_to: String,
    pub csrf_cookie_name: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    google_scopes: String,
    pub captcha_public_key: String,
    pub captcha_secrect_key: String,
    pub email_validation: bool,
    pub email_validation_redirect: String,
    pub email_default_sender: String,
    pub stripe_public_key: String,
    pub stripe_webhook_secret: String
}

impl WebsiteConfig {
    pub fn google_scopes(&self) -> Vec<Scope> {
        if self.google_scopes.is_empty() {
            vec![
                Scope::new("https://www.googleapis.com/auth/userinfo.email".into()),
                Scope::new("https://www.googleapis.com/auth/userinfo.profile".into()),
            ]
        } else {
            self.google_scopes
                .split(",")
                .map(|s| Scope::new(s.into()))
                .collect()
        }
    }
}

impl ServiceConfig for WebsiteConfig {
    fn stub() -> Self {
        Self {
            ip: Ipv4Addr::new(0, 0, 0, 0),
            port: 8000,
            domain: "test.com".into(),
            allowed_origins: "*".into(),
            session_key: "session_key".into(),
            sessions_db: "./test-sessions.sqlite".into(),
            session_cookie_name: "session_id".into(),
            csrf_cookie_name: "csrf_token".into(),
            session_expiration: 30,
            login_redirect_to: "admin".into(),
            google_client_id: "".into(),
            google_client_secret: "".into(),
            google_scopes: "scope1,scope2".into(),
            captcha_public_key: "1x00000000000000000000AA".into(),
            captcha_secrect_key: "1x0000000000000000000000000000000AA".into(),
            email_validation: false,
            email_validation_redirect: "email_validation_redirect".into(),
            email_default_sender: "email_default_sender@example.com".to_owned(),
            stripe_public_key: "stripe_public_key".into(),
            stripe_webhook_secret: "stripe_webhook_secret".into()
        }
    }

    fn socket_addr(&self) -> (Ipv4Addr, u16) {
        (self.ip, self.port)
    }

    fn allowed_origins(&self) -> AllowedOrigins {
        AllowedOrigins::from_string(&self.allowed_origins)
    }

    fn domain(&self) -> &str {
        &self.domain
    }

    fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct APIConfig {
    ip: Ipv4Addr,
    port: u16,
    domain: String,
    allowed_origins: String,
    pub session_key: String,
    pub sessions_db: String,
    pub session_cookie_name: String,
    pub session_expiration: i64,
}

impl ServiceConfig for APIConfig {
    fn stub() -> Self {
        Self {
            ip: Ipv4Addr::new(0, 0, 0, 0),
            port: 8000,
            domain: "test.com".into(),
            allowed_origins: "*".into(),
            session_key: "session_key".to_owned(),
            sessions_db: "./test-sessions.sqlite".to_owned(),
            session_cookie_name: "session_id".to_owned(),
            session_expiration: 30,
        }
    }

    fn socket_addr(&self) -> (Ipv4Addr, u16) {
        (self.ip, self.port)
    }

    fn allowed_origins(&self) -> AllowedOrigins {
        AllowedOrigins::from_string(&self.allowed_origins)
    }

    fn domain(&self) -> &str {
        &self.domain
    }

    fn port(&self) -> u16 {
        self.port
    }
}

pub trait ServiceConfig {
    fn stub() -> Self;

    fn socket_addr(&self) -> (Ipv4Addr, u16);

    fn allowed_origins(&self) -> AllowedOrigins;

    fn domain(&self) -> &str;
    fn port(&self) -> u16;

    fn print(&self) {
        println!("{}", self.build_url(""))
    }

    fn build_url(&self, path: &str) -> String {
        //TODO: maybe use reqwest url or something better
        let domain = self.domain();
        let mut url = String::new();
        if domain.starts_with("localhost")
            || domain.starts_with("127.0.0.1")
            || domain.starts_with("0.0.0.0")
        {
            url.push_str("http://");
            url.push_str(domain);
            url.push(':');
            url.push_str(self.port().to_string().as_str());
        } else {
            url.push_str("https://");
            url.push_str(domain);
        }
        url.push_str(path);
        url
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url_external() {
        let config = APIConfig {
            ip: Ipv4Addr::new(0, 0, 0, 0),
            port: 8000,
            domain: "test.com".into(),
            allowed_origins: "*".into(),
            session_key: "session_key".to_owned(),
            sessions_db: "./test-sessions.sqlite".to_owned(),
            session_cookie_name: "session_id".to_owned(),
            session_expiration: 30,
        };

        assert_eq!(config.build_url("/test"), "https://test.com/test");
    }

    #[test]
    fn test_build_url_local() {
        let config = APIConfig {
            ip: Ipv4Addr::new(0, 0, 0, 0),
            port: 8000,
            domain: "localhost".into(),
            allowed_origins: "*".into(),
            session_key: "session_key".to_owned(),
            sessions_db: "./test-sessions.sqlite".to_owned(),
            session_cookie_name: "session_id".to_owned(),
            session_expiration: 30,
        };

        assert_eq!(config.build_url("/test"), "http://localhost:8000/test");
    }

    #[test]
    fn test_build_url_what_we_do_here() {
        let config = APIConfig {
            ip: Ipv4Addr::new(192, 168, 1, 10),
            port: 8000,
            domain: "192.168.1.10".into(),
            allowed_origins: "*".into(),
            session_key: "session_key".to_owned(),
            sessions_db: "./test-sessions.sqlite".to_owned(),
            session_cookie_name: "session_id".to_owned(),
            session_expiration: 30,
        };

        assert_eq!(config.build_url("/test"), "https://192.168.1.10/test");
    }
}
