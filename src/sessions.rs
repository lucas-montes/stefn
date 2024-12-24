use crate::{
    database::Database, log_and_wrap_custom_internal, models::UserSession, service::AppError,
};
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

//TODO: check this https://docs.rs/axum/latest/axum/middleware/struct.AddExtension.html

#[derive(Clone, Debug)]
pub struct Session(Arc<RwLock<SessionData>>);

impl Session {
    fn new(data: SessionData) -> Self {
        Self(Arc::new(RwLock::new(data)))
    }

    pub async fn is_authenticated(&self, database: &Database) -> Result<bool, AppError> {
        self.0.read().await.user.is_authenticated(database).await
    }

    pub async fn set_data<T: Serialize>(&self, data: &T) -> Result<(), AppError> {
        //TODO: improve this
        let mut storage = self.0.write().await;
        storage.data =
            Some(serde_json::to_vec(data).map_err(|e| log_and_wrap_custom_internal!(e))?);
        Ok(())
    }

    pub async fn get_data<T: DeserializeOwned>(&self) -> Result<T, AppError> {
        serde_json::from_slice(self.0.read().await.data.as_ref().unwrap())
            .map_err(|e| log_and_wrap_custom_internal!(e))
    }

    pub async fn user_pk(&self) -> Option<i64> {
        self.0.read().await.user.pk()
    }

    pub async fn id(&self) -> String {
        self.0.read().await.session_id.to_owned()
    }

    pub async fn csrf_token(&self) -> String {
        self.0.read().await.csrf_token.to_owned()
    }

    pub async fn validate_csrf_token(&self, secret: &str, token: &str) -> Result<(), AppError> {
        //TODO: use the constant time comparaison think to avoid some security problem
        if generate_token(secret, &self.0.read().await.get_token_data()).eq(token) {
            Ok(())
        } else {
            Err(AppError::Unauthorized)
        }
    }
}

#[derive(Clone, Debug)]
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

    pub async fn find_session(
        &self,
        session_id: &str,
        secret: &str,
    ) -> Result<Option<Session>, AppError> {
        SessionData::from_session_id(session_id, self.get_connection())
            .await
            .map(|u| {
                u.map(|mut s| {
                    s.update_csrf_token(secret);
                    Session::new(s)
                })
            })
    }

    pub async fn create_session(
        &self,
        user: UserSession,
        session_expiration: u64,
        secret: &str,
        country: Option<String>,
    ) -> Result<Session, AppError> {
        let session = SessionData::new(user, country, session_expiration, secret);
        session.save(self.get_connection()).await?;
        Ok(Session(Arc::new(RwLock::new(session))))
    }

    pub async fn reuse_current_as_new_one(
        &self,
        session: &Session,
        user: UserSession,
        secret: &str,
    ) -> Result<(), AppError> {
        session
            .0
            .write()
            .await
            .delete(self.get_connection())
            .await?
            .new_session_id()
            .update_dates()
            .update_csrf_token(secret)
            //TODO: the orden of the date + token is important so the token has the new date. not pretty
            .update_user(user)
            .save(self.get_connection())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, sqlx::FromRow, Clone)]
pub struct SessionData {
    session_id: String,
    #[sqlx(flatten)]
    user: UserSession,
    last_accessed: NaiveDateTime,
    created_at: NaiveDateTime,
    expiration: NaiveDateTime,
    #[sqlx(skip)]
    csrf_token: String,
    data: Option<Vec<u8>>,
    country: Option<String>,
}

impl SessionData {
    fn new(
        user: UserSession,
        country: Option<String>,
        session_expiration: u64,
        secret: &str,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        let today = DateTime::from_timestamp(now, 0).unwrap().naive_utc();
        let expiration = today + Days::new(session_expiration);
        let mut session = Self {
            session_id: Uuid::now_v7().to_string(),
            user,
            last_accessed: today,
            created_at: today,
            expiration,
            csrf_token: String::default(),
            data: None,
            country: country.into(),
        };
        session.update_csrf_token(secret);
        //TODO: improve how token is created and set. this is a little convoluted
        session
    }

    fn get_token_data(&self) -> String {
        format!("{}-{}", self.session_id, self.created_at)
    }

    fn new_session_id(&mut self) -> &mut Self {
        self.session_id = Uuid::now_v7().to_string();
        self
    }

    fn update_user(&mut self, user: UserSession) -> &mut Self {
        self.user = user;
        self
    }

    fn update_csrf_token(&mut self, secret: &str) -> &mut Self {
        self.csrf_token = generate_token(secret, &self.get_token_data());
        self
    }

    fn update_dates(&mut self) -> &mut Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        let today = DateTime::from_timestamp(now, 0).unwrap().naive_utc();
        self.last_accessed = today;
        self.created_at = today;
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
            .map_err(|e| log_and_wrap_custom_internal!(e))
    }

    async fn delete(&mut self, conn: &SqlitePool) -> Result<&mut Self, AppError> {
        sqlx::query("DELETE FROM web_sessions WHERE session_id = $1;")
            .bind(&self.session_id)
            .execute(conn)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        Ok(self)
    }

    async fn save(&self, conn: &SqlitePool) -> Result<i64, AppError> {
        sqlx::query("INSERT INTO web_sessions(session_id, user_pk, groups, last_accessed, created_at, expiration, data, country) VALUES ($1, $2, $3, $4, $5, $6, $7, $8);")
            .bind(&self.session_id)
            .bind(self.user.pk())
            .bind(self.user.groups().map(|u|u.to_string()))
            .bind(&self.last_accessed)
            .bind(&self.created_at)
            .bind(&self.expiration)
            .bind(&self.data)
            .bind(&self.country)
            .execute(conn)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
            .map(|r|r.last_insert_rowid())
    }
}

fn generate_token(secret: &str, data: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data.as_bytes());

    hex::encode(mac.finalize().into_bytes())
}
