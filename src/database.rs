use std::str::FromStr;

use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, SqlitePool};

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

pub struct IpsDatabase {
    storage: Option<maxminddb::Reader<Vec<u8>>>,
}

impl IpsDatabase {
    pub fn new(url: &str) -> Self {
        let storage = if url.is_empty() {
            None
        } else {
            Some(
                maxminddb::Reader::open_readfile(url)
                    .expect("the database for the ips seems to be missing or is the wrong path"),
            )
        };
        Self { storage }
    }
}
