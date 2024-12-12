use std::{
    error::Error,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, sqlite::SqliteRow, Decode, Row, SqliteConnection, Type};

use crate::{database::Database, service::AppError};

#[derive(Type, Debug, Copy, Clone, Deserialize, Serialize, PartialEq)]
#[repr(i64)]
pub enum Group {
    Admin = 1,
    User = 2,
}

impl FromStr for Group {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "Admin" | "1" => Ok(Self::Admin),
            "User" | "2" => Ok(Self::User),
            _ => Err(()),
        }
    }
}

impl Group {
    fn name(&self) -> &str {
        match self {
            Self::Admin => "Admin",
            Self::User => "User",
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct Groups(Vec<Group>);

impl FromRow<'_, SqliteRow> for Groups {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let groups: String = row.try_get("groups")?;
        Ok(groups.parse().unwrap())
    }
}

impl<DB: sqlx::Database> Type<DB> for Groups {
    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        todo!()
    }
}

impl<'r, DB: sqlx::Database> Decode<'r, DB> for Groups
where
    // we want to delegate some of the work to string decoding so let's make sure strings
    // are supported by the database
    &'r str: Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::Database>::ValueRef<'r>,
    ) -> Result<Self, Box<dyn Error + 'static + Send + Sync>> {
        // the interface of ValueRef is largely unstable at the moment
        // so this is not directly implementable

        // however, you can delegate to a type that matches the format of the type you want
        // to decode (such as a UTF-8 string)

        let value = <&str as Decode<DB>>::decode(value)?;

        // now you can parse this into your type (assuming there is a `FromStr`)

        Ok(value.parse()?)
    }
}

impl FromStr for Groups {
    type Err = Box<dyn Error + 'static + Send + Sync>;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            input
                .split(",")
                .filter_map(|g| g.parse::<Group>().ok())
                .collect(),
        ))
    }
}

impl Groups {
    pub fn to_string(&self) -> String {
        self.0
            .iter()
            .filter_map(|&g| std::char::from_digit(g as u32, 10))
            .collect()
    }

    fn push(&mut self, group: Group) {
        self.0.push(group);
    }

    fn contains(&self, group: &Group) -> bool {
        self.0.contains(group)
    }
}

#[derive(Type, Debug, Deserialize, Serialize, Clone, Default)]
pub struct UserSession(Option<User>);

impl FromRow<'_, SqliteRow> for UserSession {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        if row.is_empty() {
            Ok(Self(None))
        } else {
            Ok(Self(Some(User::from_row(row)?)))
        }
    }
}

impl UserSession {
    pub fn groups(&self) -> Option<&Groups> {
        self.0.as_ref().map(|u| &u.groups)
    }

    pub fn pk(&self) -> Option<i64> {
        self.0.as_ref().map(|u| u.pk)
    }

    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }
}

#[derive(FromRow, Type, Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub pk: i64,
    pub groups: Groups,
}
// TODO: by default start with a user like this one to manage state and auth. Create a nice trait
// and make it public so you can hook your own auth/state struct

impl User {
    pub fn for_session(self) -> UserSession {
        UserSession(Some(self))
    }
    pub async fn create(tx: &mut SqliteConnection, password: &str) -> Result<Self, AppError> {
        let pk = sqlx::query("INSERT INTO users (password) VALUES ($1);")
            .bind(password)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
            .map(|q| q.last_insert_rowid())?;
        Ok(Self {
            pk,
            groups: Groups::default(),
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
            groups: Groups::default(),
        })
    }

    pub async fn add_to_group(
        mut self,
        group: Group,
        tx: &mut SqliteConnection,
    ) -> Result<Self, AppError> {
        if self.groups.contains(&group) {
            return Ok(self);
        }

        sqlx::query("INSERT INTO users_groups_m2m (user_pk,  group_pk) VALUES ($1, $2);")
            .bind(self.pk)
            .bind(group.clone() as i64)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))?;

        self.groups.push(group);
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

    pub async fn set_to_active(&self, tx: &mut SqliteConnection) -> Result<i64, AppError> {
        sqlx::query("UPDATE users SET is_active = $1 WHERE pk = 2$;")
            .bind(1)
            .bind(self.pk)
            .execute(tx)
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))
            .map(|q| q.last_insert_rowid())
    }

    pub async fn find_by_email_with_password(
        database: &Database,
        email: &str,
    ) -> Result<Option<Self>, AppError> {
        sqlx::query_as(
            "SELECT users.pk as user_pk, GROUP_CONCAT(group_pk, ',') as groups, users.password
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
}

#[derive(Debug, FromRow)]
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
            "SELECT emails.pk, emails.user_pk, GROUP_CONCAT(group_pk, ',') as groups 
            FROM emails
            LEFT JOIN users_groups_m2m ON users_groups_m2m.user_pk = emails.user_pk
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
        assert!(user.groups.contains(&Group::Admin));
        assert!(!user.groups.contains(&Group::User));
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
        assert!(user.groups.contains(&Group::Admin));
        assert!(user.groups.contains(&Group::User));
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
