use crate::AppError;
use chrono::{DateTime, Days, NaiveDateTime};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, SqlitePool};
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct Sessions(SqlitePool);

impl Sessions {
    pub fn new(sessions_db: &str) -> Self {
        let database_config = SqliteConnectOptions::from_str(sessions_db)
            .expect("Cannot connect to database")
            .create_if_missing(true);

        Self(SqlitePool::connect_lazy_with(database_config))
    }

    pub async fn run_migrations(&self) {
        Migrator::new(std::path::Path::new("./migrations/sessions"))
            .await
            .expect("Where are the migrations?")
            .run(&self.0)
            .await
            .expect("Migrations failed");
    }

    pub fn get_connection(&self) -> &SqlitePool {
        &self.0
    }

    pub async fn find_session(&self, session_id: &str) -> Result<Option<Session>, AppError> {
        Session::from_session_id(session_id, self.get_connection()).await
    }

    pub async fn create_session(
        &self,
        user_pk: i64,
        groups: String,
        session_expiration: u64,

        country: &str,
    ) -> Result<Session, AppError> {
        let session = Session::new(user_pk, groups, country, session_expiration);

        session.save(self.get_connection()).await?;
        Ok(session)
    }
}

#[derive(Deserialize, sqlx::FromRow, Clone)]
pub struct Session {
    session_id: Uuid,
    user_pk: i64,
    groups: String,
    last_accessed: NaiveDateTime,
    expiration: NaiveDateTime,
    csrf_token: String,
    data: Option<Vec<u8>>,
    country: String,
}

impl Session {
    pub fn id(&self) -> &Uuid {
        &self.session_id
    }

    pub fn expiration_offset(&self) {}

    fn new(user_pk: i64, groups: String, country: &str, session_expiration: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        let today = DateTime::from_timestamp(now, 0).unwrap().naive_utc();
        let expiration = today + Days::new(60 * 60 * 24 * session_expiration);
        Session {
            session_id: Uuid::now_v7(),
            user_pk,
            groups,
            last_accessed: today,
            expiration,
            csrf_token: String::new(),
            data: None,
            country: country.into(),
        }
    }

    fn get_token_data(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            self.session_id, self.user_pk, self.country, self.last_accessed
        )
    }

    pub fn new_csrf_token(mut self, secret: &str) -> Self {
        self.csrf_token = generate_token(secret, &self.get_token_data());
        self
    }

    fn validate_token() {
        fn calculate_hmac(secret: &str, data: &str) -> String {
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(data.as_bytes());

            hex::encode(mac.finalize().into_bytes())
        }
    }

    async fn from_session_id(
        session_id: &str,
        conn: &SqlitePool,
    ) -> Result<Option<Session>, AppError> {
        sqlx::query_as("SELECT * FROM web_sessions WHERE pk = $1")
            .bind(session_id)
            .fetch_optional(conn)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
    }

    async fn save(&self, conn: &SqlitePool) -> Result<i64, AppError> {
        sqlx::query("INSERT INTO web_sessions(session_id, user_pk, groups, last_accessed, expiration, csrf_token, data, country) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(self.session_id)
            .bind(self.user_pk)
            .bind(&self.groups)
            .bind(self.last_accessed)
            .bind(self.expiration)
            .bind(&self.csrf_token)
            .bind(&self.data)
            .bind(&self.country)
            .execute(conn)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
            .map(|r|r.last_insert_rowid())
    }
}

fn generate_token(secret: &str, data: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data.as_bytes());

    hex::encode(mac.finalize().into_bytes())
}
