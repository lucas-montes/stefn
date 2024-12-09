use std::{net::SocketAddr, str::FromStr, sync::Arc};

use maxminddb::geoip2;
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, Sqlite, SqlitePool, Transaction};

use crate::service::AppError;

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

    pub fn stub() -> Self {
        let database_config = SqliteConnectOptions::from_str("test-database.sqlite")
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

    pub fn get_connection(&self) -> &SqlitePool {
        &self.storage
    }

    pub async fn start_transaction(&self) -> Result<Transaction<'_, Sqlite>, AppError> {
        self.get_connection()
            .begin()
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
    }
}

pub struct TestDatabase(Database);

impl TestDatabase {
    pub async fn setup() -> Self {
        let database = Database::stub();
        database.run_migrations().await;
        Self(database)
    }

    pub fn database(&self) -> &Database {
        &self.0
    }

    pub fn get_connection(&self) -> &SqlitePool {
        &self.0.storage
    }

    pub async fn start_transaction(&self) -> Result<Transaction<'_, Sqlite>, AppError> {
        self.get_connection()
            .begin()
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
    }

    pub async fn run_test_migrations(&self, path: &str) {
        Migrator::new(std::path::Path::new(path))
            .await
            .expect("Where are the migrations?")
            .run(&self.0.storage)
            .await
            .expect("Migrations failed");
    }

    pub async fn clean_database(&self) -> Result<(), AppError> {
        let mut tx = self.0.start_transaction().await?;

        // Query to retrieve all user-defined tables
        let tables: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT name 
            FROM sqlite_master 
            WHERE type='table' AND name NOT LIKE 'sqlite_%';
            "#,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| AppError::custom_internal(&format!("Failed to fetch table names: {}", e)))?;

        // Delete all data from tables
        for table in &tables {
            sqlx::query(&format!("DELETE FROM {};", table))
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    AppError::custom_internal(&format!("Failed to clean table {}: {}", table, e))
                })?;
        }

        // Reset auto-increment counters
        for table in &tables {
            sqlx::query(&format!(
                "DELETE FROM sqlite_sequence WHERE name = '{}';",
                table
            ))
            .execute(&mut *tx)
            .await
            .ok(); // Ignore errors if the table has no auto-increment sequence
        }

        tx.commit()
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))?;

        Ok(())
    }
}

// impl Drop for TestDatabase {
//     fn drop(&mut self) {
//         tokio::runtime::Handle::current().spawn_blocking(async {
//             clean_database(&self.database).await.unwrap_or_else(|e| {
//                 eprintln!("Failed to clean database during teardown: {:?}", e);
//             });
//         })
//     }
// }

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
