use std::{net::SocketAddr, str::FromStr, sync::Arc};

use maxminddb::geoip2;
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, SqlitePool};

use crate::AppError;

#[derive(Clone)]
pub struct Database {
    storage: SqlitePool,
}

impl Database {
    pub fn new(url: &str) -> Self {
        let database_config = SqliteConnectOptions::from_str(url)
            .expect("Cannot connect to database")
            .create_if_missing(true);

        Self {
            storage: SqlitePool::connect_lazy_with(database_config),
        }
    }

    pub async fn run_migrations(&self) {
        Migrator::new(std::path::Path::new("./migrations/principal"))
            .await
            .expect("Where are the migrations?")
            .run(&self.storage)
            .await
            .expect("Migrations failed");
    }

    pub async fn get_connection(&self) -> &SqlitePool {
        &self.storage
    }
}

#[derive(Clone)]
pub struct IpsDatabase {
    storage: Option<Arc<maxminddb::Reader<Vec<u8>>>>,
}

impl IpsDatabase {
    pub fn new(url: &str) -> Self {
        let storage = if url.is_empty() {
            None
        } else {
            Some(Arc::new(maxminddb::Reader::open_readfile(url).expect(
                "the database for the ips seems to be missing or is the wrong path",
            )))
        };
        Self { storage }
    }

    pub fn get_country_code_from_ip(&self, addr: SocketAddr) -> Result<&str, AppError> {
        if addr.ip().is_loopback() {
            return Ok("ES");
        }
        self.storage
            .as_ref()
            .unwrap()
            .lookup::<geoip2::City>(addr.ip())
            .map_err(AppError::IpError)?
            .country
            .ok_or(AppError::IpDataNotFound)?
            .iso_code
            .ok_or(AppError::IpDataNotFound)
    }
}
