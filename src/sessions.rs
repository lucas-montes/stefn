use crate::AppError;
use chrono::{DateTime, Days, NaiveDateTime};
use hmac::{Hmac, Mac};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha256;
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, SqlitePool};
use std::{
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct Session(Arc<RwLock<UserSession>>);

impl Session {
    pub async fn is_authenticated(&self) -> bool {
        self.0.read().await.user_pk.is_some()
    }

    pub async fn set_data<T: Serialize>(&self, data: &T) -> Result<(), AppError> {
        //TODO: improve this
        let mut storage = self.0.write().await;
        storage.data =
            Some(serde_json::to_vec(data).map_err(|e| AppError::custom_internal(&e.to_string()))?);
        Ok(())
    }

    pub async fn get_data<T: DeserializeOwned>(&self) -> Result<T, AppError> {
        serde_json::from_slice(self.0.read().await.data.as_ref().unwrap())
            .map_err(|e| AppError::custom_internal(&e.to_string()))
    }

    pub async fn user_pk(&self) -> Option<i64> {
        self.0.read().await.user_pk
    }

    pub async fn id(&self) -> String {
        self.0.read().await.session_id.to_owned()
    }

    pub async fn csrf_token(&self) -> String {
        self.0.read().await.csrf_token.to_owned()
    }

    pub async fn token_is_valid(&self, secret: &str, token: &str) -> bool {
        generate_token(secret, &self.0.read().await.get_token_data()).eq(token)
    }
}

#[derive(Clone)]
pub struct Sessions(SqlitePool);
//TODO: do we really need a second database?

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
        UserSession::from_session_id(session_id, self.get_connection())
            .await
            .map(|u| u.map(|u| Session(Arc::new(RwLock::new(u)))))
    }

    pub async fn create_session(
        &self,
        user_pk: Option<i64>,
        groups: String,
        session_expiration: u64,

        country: &str,
    ) -> Result<Session, AppError> {
        let session = UserSession::new(user_pk, groups, country, session_expiration);

        session.save(self.get_connection()).await?;
        Ok(Session(Arc::new(RwLock::new(session))))
    }

    pub async fn update_session(
        &self,
        session: Session,
        secret: &str,
    ) -> Result<Session, AppError> {
        session
            .0
            .write()
            .await
            .update_last_accessed()
            .update_csrf_token(secret)
            .update(self.get_connection())
            .await?;

        Ok(session)
    }

    pub async fn reuse_current_as_new_one(
        &self,
        session: Session,
        user_pk: i64,
        groups: String,
    ) -> Result<(), AppError> {
        //TODO: change this interface, take a struct as user
        session
            .0
            .write()
            .await
            .new_session_id()
            .update_user(user_pk, groups)
            .save(self.get_connection())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, sqlx::FromRow, Clone)]
pub struct UserSession {
    session_id: String,
    user_pk: Option<i64>,
    groups: String,
    last_accessed: NaiveDateTime,
    expiration: NaiveDateTime,
    csrf_token: String,
    data: Option<Vec<u8>>,
    country: String,
}
// TODO: replace data, user_pk and groups by a struct that would hold user info plus additional info

impl UserSession {
    fn new(user_pk: Option<i64>, groups: String, country: &str, session_expiration: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        let today = DateTime::from_timestamp(now, 0).unwrap().naive_utc();
        let expiration = today + Days::new(session_expiration);
        Self {
            session_id: Uuid::now_v7().to_string(),
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
        format!("{}-{}", self.session_id, self.last_accessed)
    }

    fn token_is_valid(&self, secret: &str, token: &str) -> bool {
        generate_token(secret, &self.get_token_data()).eq(token)
    }

    fn new_session_id(&mut self) -> &mut Self {
        self.session_id = Uuid::now_v7().to_string();
        self
    }

    fn update_user(&mut self, user_pk: i64, groups: String) -> &mut Self {
        self.user_pk = Some(user_pk);
        self.groups = groups;
        self
    }

    fn update_csrf_token(&mut self, secret: &str) -> &mut Self {
        self.csrf_token = generate_token(secret, &self.get_token_data());
        self
    }

    fn update_last_accessed(&mut self) -> &mut Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        self.last_accessed = DateTime::from_timestamp(now, 0).unwrap().naive_utc();
        self
    }

    async fn from_session_id(
        session_id: &str,
        conn: &SqlitePool,
    ) -> Result<Option<Self>, AppError> {
        sqlx::query_as("SELECT * FROM web_sessions WHERE session_id = $1;")
            .bind(session_id)
            .fetch_optional(conn)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
    }

    async fn save(&self, conn: &SqlitePool) -> Result<i64, AppError> {
        sqlx::query("INSERT INTO web_sessions(session_id, user_pk, groups, last_accessed, expiration, csrf_token, data, country) VALUES ($1, $2, $3, $4, $5, $6, $7, $8);")
            .bind(&self.session_id)
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

    async fn update(&self, conn: &SqlitePool) -> Result<i64, AppError> {
        sqlx::query(
            "UPDATE web_sessions SET last_accessed = $1, csrf_token = $2 WHERE session_id = $3;",
        )
        .bind(self.last_accessed)
        .bind(&self.csrf_token)
        .bind(&self.session_id)
        .execute(conn)
        .await
        .map_err(|e| AppError::custom_internal(&e.to_string()))
        .map(|r| r.last_insert_rowid())
    }
}

fn generate_token(secret: &str, data: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data.as_bytes());

    hex::encode(mac.finalize().into_bytes())
}
