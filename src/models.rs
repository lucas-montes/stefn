use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, SqliteConnection};

use crate::{AppError, Database};

#[derive(Clone)]
pub enum Group {
    Admin = 1,
    User = 2,
}

impl Group {
    fn to_str(&self) -> &str {
        match self {
            Self::Admin => "Admin",
            Self::User => "User",
        }
    }
}
#[derive(FromRow, Deserialize, Serialize)]
pub struct User {
    #[sqlx(rename = "user_pk")]
    pub pk: i64,
    pub groups: String,
}
// TODO: by default start with a user like this one to manage state and auth. Create a nice trait
// and make it public so you can hook your own auth/state struct

impl User {
    pub async fn create_active(tx: &mut SqliteConnection) -> Result<Self, AppError> {
        let pk = sqlx::query("INSERT INTO users (password, is_active) VALUES ($1, $2);")
            .bind("SDFdso34$hl#sdfj")
            .bind(1)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
            .map(|q| q.last_insert_rowid())?;
        Ok(Self {
            pk,
            groups: String::default(),
        })
    }

    pub async fn add_to_group(
        mut self,
        group: Group,
        tx: &mut SqliteConnection,
    ) -> Result<Self, AppError> {
        if self.groups.contains(group.to_str()) {
            return Ok(self);
        }

        sqlx::query("INSERT INTO users_groups_m2m (user_pk,  group_pk) VALUES ($1, $2);")
            .bind(self.pk)
            .bind(group.clone() as i64)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))?;

        self.groups.push_str(group.to_str());
        self.groups.push(',');
        Ok(self)
    }

    pub async fn add_profile(
        &self,
        tx: &mut SqliteConnection,
        name: &str,
        given_name: &str,
        family_name: &str,
        picture: &str,
    ) -> Result<i64, AppError> {
        sqlx::query("INSERT INTO profiles (name, given_name, family_name, picture, user_pk) VALUES ($1, $2, $3, $4, $5);")
            .bind(name)
            .bind(given_name)
            .bind(family_name)
            .bind(picture)
            .bind(self.pk)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
            .map(|q| q.last_insert_rowid())
    }
}

#[derive(FromRow)]
pub struct EmailAccount {
    pub pk: i64,
    #[sqlx(flatten)]
    pub user: User,
}

impl EmailAccount {
    pub async fn find(database: &Database, email: &str) -> Result<Option<Self>, AppError> {
        sqlx::query_as(
            "SELECT emails.pk, pk as user_pk, GROUP_CONCAT(group_pk, ',') as groups 
                FROM emails
                INNER JOIN users ON users.email_pk = emails.pk
                LEFT JOIN users_groups_m2m ON users.pk = users_groups_m2m.user_pk
                WHERE emails.email = $1;",
        )
        .bind(email)
        .fetch_optional(database.get_connection())
        .await
        .map_err(|e| AppError::custom_internal(&e.to_string()))
    }

    pub async fn create_primary_active(
        tx: &mut SqliteConnection,
        user: User,
        email: &str,
    ) -> Result<Self, AppError> {
        let activated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        let pk = sqlx::query("INSERT INTO emails (is_primary, user_pk, activated_at, email) VALUES ($1, $2, $3, $4);")
            .bind(1)
            .bind(user.pk)
            .bind(activated_at)
            .bind(email)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
            .map(|q| q.last_insert_rowid())?;
        Ok(Self { pk, user })
    }
}
