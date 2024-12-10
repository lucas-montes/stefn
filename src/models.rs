use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, SqliteConnection};

use crate::{database::Database, service::AppError};

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
    // TODO: create custom type for groups
}
// TODO: by default start with a user like this one to manage state and auth. Create a nice trait
// and make it public so you can hook your own auth/state struct

impl User {
    pub async fn create(tx: &mut SqliteConnection, password: &str) -> Result<Self, AppError> {
        let pk = sqlx::query("INSERT INTO users (password) VALUES ($1);")
            .bind(password)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
            .map(|q| q.last_insert_rowid())?;
        Ok(Self {
            pk,
            groups: String::default(),
        })
    }

    pub async fn create_active_default(tx: &mut SqliteConnection) -> Result<Self, AppError> {
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

    pub async fn set_to_active(self, tx: &mut SqliteConnection) -> Result<Self, AppError> {
        sqlx::query("UPDATE users SET is_active = $1 WHERE pk = 2$;")
            .bind(1)
            .bind(self.pk)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
            .map(|q| q.last_insert_rowid())?;
        Ok(self)
    }
}

#[derive(FromRow)]
pub struct EmailAccount {
    pub pk: i64,
    #[sqlx(flatten)]
    pub user: User,
}

impl EmailAccount {
    pub async fn get_by_email(database: &Database, email: &str) -> Result<Option<Self>, AppError> {
        sqlx::query_as(
            "SELECT emails.pk, users.pk as user_pk, GROUP_CONCAT(group_pk, ',') as groups 
                FROM emails
                INNER JOIN users ON users.pk = emails.user_pk
                LEFT JOIN users_groups_m2m ON users.pk = users_groups_m2m.user_pk
                WHERE emails.email = $1;",
        )
        .bind(email)
        .fetch_optional(database.get_connection())
        .await
        .map_err(|e| AppError::custom_internal(&e.to_string()))
    }

    pub async fn get_by_pk(tx: &mut SqliteConnection, pk: i64) -> Result<Self, AppError> {
        sqlx::query_as(
            "SELECT emails.pk, pk as user_pk, GROUP_CONCAT(group_pk, ',') as groups 
                FROM emails
                INNER JOIN users ON users.pk = emails.pk
                LEFT JOIN users_groups_m2m ON users.pk = users_groups_m2m.user_pk
                WHERE emails.pk = $1;",
        )
        .bind(pk)
        .fetch_one(tx)
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
    pub async fn create_primary(
        tx: &mut SqliteConnection,
        user: User,
        email: &str,
    ) -> Result<Self, AppError> {
        let pk =
            sqlx::query("INSERT INTO emails (is_primary, user_pk, email) VALUES ($1, $2, $3);")
                .bind(1)
                .bind(user.pk)
                .bind(email)
                .execute(tx)
                .await
                .map_err(|e| AppError::custom_internal(&e.to_string()))
                .map(|q| q.last_insert_rowid())?;
        Ok(Self { pk, user })
    }

    pub async fn set_to_active(self, tx: &mut SqliteConnection) -> Result<Self, AppError> {
        let activated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        sqlx::query("UPDATE emails SET activated_at = $1 WHERE pk = 2$;")
            .bind(activated_at)
            .bind(self.pk)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
            .map(|q| q.last_insert_rowid())?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::TestDatabase;

    use super::*;

    #[tokio::test]
    async fn test_create_active_default() {
        let database = TestDatabase::setup().await;

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut tx).await;
        tx.commit().await.unwrap();

        assert!(user.is_ok());
        let user_pk = user.unwrap().pk;
        assert!(user_pk > 0);
    }

    #[tokio::test]
    async fn test_add_profile() {
        let database = TestDatabase::setup().await;

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut tx)
            .await
            .unwrap()
            .add_profile(&mut tx, "name", "given_name", "family_name", "picture")
            .await;
        tx.commit().await.unwrap();

        assert!(user.is_ok());
        let profile_pk = user.unwrap();
        assert!(profile_pk > 0);
    }

    #[tokio::test]
    async fn test_create_admin_user() {
        let database = TestDatabase::setup().await;

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut tx).await;

        assert!(user.is_ok());
        let user = user.unwrap().add_to_group(Group::Admin, &mut tx).await;
        tx.commit().await.unwrap();
        assert!(user.is_ok());
        let user = user.unwrap();
        assert!(user.pk > 0);
        assert_eq!(user.groups, "Admin,");
    }

    #[tokio::test]
    async fn test_create_user_multiple_groups() {
        let database = TestDatabase::setup().await;

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut tx)
            .await
            .unwrap()
            .add_to_group(Group::Admin, &mut tx)
            .await
            .unwrap()
            .add_to_group(Group::User, &mut tx)
            .await
            .unwrap()
            .add_to_group(Group::Admin, &mut tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert!(user.pk > 0);
        assert_eq!(user.groups, "Admin,User,");
    }

    #[tokio::test]
    async fn test_create_email_account() {
        let database = TestDatabase::setup().await;

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut tx).await.unwrap();
        let email_account = EmailAccount::create_primary_active(
            &mut tx,
            user,
            "test_create_email_account@example.com",
        )
        .await;
        tx.commit().await.unwrap();

        assert!(email_account.is_ok());
        let account = email_account.unwrap();
        assert!(account.pk > 0);

        let result: Option<(i64, i64, String)> =
            sqlx::query_as("SELECT pk, user_pk, email FROM emails WHERE pk = $1;")
                .bind(account.pk)
                .fetch_optional(database.get_connection())
                .await
                .unwrap();

        assert!(result.is_some());
        let (db_pk, db_user_pk, db_email) = result.unwrap();
        assert_eq!(db_pk, account.pk);
        assert_eq!(db_user_pk, account.user.pk);
        assert_eq!(db_email, "test_create_email_account@example.com");

        sqlx::query("DELETE FROM users WHERE pk = $1;")
            .bind(account.user.pk)
            .execute(database.get_connection())
            .await
            .unwrap();
    }
}
