use std::{net::SocketAddr, ops::Deref, str::FromStr, sync::Arc};

use maxminddb::geoip2;
use sqlx::{migrate::Migrator, postgres::PgConnectOptions, PgPool, Postgres, Transaction};

use crate::{log_and_wrap_custom_internal, service::AppError};

#[derive(Clone, Debug)]
pub struct Database(PgPool);

impl Deref for Database {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Database {
    pub fn new(url: &str) -> Database {
        let database_config = PgConnectOptions::from_str(url)
            .expect("Cannot connect to database")
            .application_name(env!("CARGO_PKG_NAME"));

        Self(PgPool::connect_lazy_with(database_config))
    }

    pub async fn run_migrations(&self) {
        Migrator::new(std::path::Path::new("./migrations/principal"))
            .await
            .expect("Where are the migrations?")
            .run(&**self)
            .await
            .expect("Migrations failed");
    }

    pub async fn start_transaction(&self) -> Result<Transaction<'_, Postgres>, AppError> {
        self.begin()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
    }
}

pub struct TestDatabase(Database);

impl Deref for TestDatabase {
    type Target = Database;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TestDatabase {
    pub async fn setup() -> Self {
        let database = Database::new("test-database");
        database.run_migrations().await;
        Self(database)
    }

    pub fn database(&self) -> &Database {
        &self.0
    }

    pub async fn start_transaction(&self) -> Result<Transaction<'_, Postgres>, AppError> {
        self.0.start_transaction().await
    }
}

#[derive(Clone, Debug)]
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

    pub fn get_country_code_from_ip(&self, addr: &SocketAddr) -> Result<&str, AppError> {
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
