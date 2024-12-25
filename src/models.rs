use std::{
    error::Error,
    fmt,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, sqlite::SqliteRow, Decode, Row, SqliteConnection, Type};

use crate::{database::Database, log_and_wrap_custom_internal, service::AppError};

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
    pub fn name(&self) -> &str {
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

impl<DB: sqlx::Database> Type<DB> for Groups
where
    String: Type<DB>,
{
    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        <String as Type<DB>>::type_info()
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

        value.parse()
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

impl fmt::Display for Groups {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r: String = self
            .0
            .iter()
            .filter_map(|&g| std::char::from_digit(g as u32, 10))
            .collect();
        write!(f, "{}", r)
    }
}

impl Groups {
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
        let pk: Option<i64> = row.try_get("user_pk")?;
        if row.is_empty() || pk.is_none() {
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

    pub async fn is_authenticated(&self, database: &Database) -> Result<bool, AppError> {
        match &self.0 {
            Some(user) => user.is_authenticated(database).await,
            None => Ok(false),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct UserWithPassword {
    #[sqlx(flatten)]
    pub user: User,
    pub password: String,
}

#[derive(FromRow, Debug, Deserialize, Serialize, Clone)]
pub struct User {
    #[sqlx(rename = "user_pk")]
    pub pk: i64,
    pub groups: Groups,
}
// TODO: by default start with a user like this one to manage state and auth. Create a nice trait
// and make it public so you can hook your own auth/state struct

impl User {
    pub fn for_session(self) -> UserSession {
        UserSession(Some(self))
    }

    pub async fn is_authenticated(&self, database: &Database) -> Result<bool, AppError> {
        let exists: (i64,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM users WHERE pk = $1 AND activated_at IS NOT NULL);",
        )
        .bind(self.pk)
        .fetch_one(database.get_connection())
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))?;

        Ok(exists.0 == 1)
    }

    pub async fn create(
        tx: &mut SqliteConnection,
        password: &str,
        activated_at: Option<i64>,
    ) -> Result<Self, AppError> {
        let pk = sqlx::query("INSERT INTO users (password, activated_at) VALUES ($1, $2);")
            .bind(password)
            .bind(activated_at)
            .execute(tx)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
            .map(|q| q.last_insert_rowid())?;
        Ok(Self {
            pk,
            groups: Groups::default(),
        })
    }

    pub async fn create_active_default(tx: &mut SqliteConnection) -> Result<Self, AppError> {
        Self::create_active(tx, "SDFdso34$hl#sdfj").await
    }

    pub async fn create_active(
        tx: &mut SqliteConnection,
        password: &str,
    ) -> Result<Self, AppError> {
        let activated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        Self::create(tx, password, Some(activated_at)).await
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
            .bind(group as i64)
            .execute(tx)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        self.groups.push(group);
        Ok(self)
    }

    pub async fn set_to_active(&self, tx: &mut SqliteConnection) -> Result<i64, AppError> {
        let activated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        sqlx::query("UPDATE users SET activated_at = $1 WHERE pk = $2;")
            .bind(activated_at)
            .bind(self.pk)
            .execute(tx)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
            .map(|q| q.last_insert_rowid())
    }

    pub async fn find_by_email_with_password(
        database: &Database,
        email: &str,
    ) -> Result<Option<UserWithPassword>, AppError> {
        sqlx::query_as(
            "SELECT emails.user_pk, GROUP_CONCAT(users_groups_m2m.group_pk, ',') as groups, users.password
                FROM emails
                INNER JOIN users ON users.pk = emails.user_pk
                LEFT JOIN users_groups_m2m ON emails.user_pk = users_groups_m2m.user_pk
                WHERE emails.email = $1;",
        )
        .bind(email)
        .fetch_optional(database.get_connection())
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))
    }
}

#[derive(Debug, FromRow)]
pub struct EmailAccount {
    pub pk: i64,
    #[sqlx(flatten)]
    pub user: User,
    pub email: String,
    //TODO: use the Email type in smartlink?
}

impl EmailAccount {
    pub fn mail_server(&self) -> &str {
        self.email.split_once("@").unwrap().1
    }

    pub fn username(&self) -> &str {
        self.email.split_once("@").unwrap().0
    }

    pub async fn get_by_email(database: &Database, email: &str) -> Result<Option<Self>, AppError> {
        sqlx::query_as(
            "SELECT emails.pk, emails.user_pk, emails.email, GROUP_CONCAT(group_pk, ',') as groups 
                FROM emails
                LEFT JOIN users_groups_m2m ON users_groups_m2m.user_pk = emails.user_pk
                WHERE emails.email = $1;",
        )
        .bind(email)
        .fetch_optional(database.get_connection())
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))
    }

    pub async fn get_by_pk(tx: &mut SqliteConnection, pk: i64) -> Result<Self, AppError> {
        sqlx::query_as(
            "SELECT emails.pk, emails.user_pk, emails.email, GROUP_CONCAT(group_pk, ',') as groups 
            FROM emails
            LEFT JOIN users_groups_m2m ON users_groups_m2m.user_pk = emails.user_pk
            WHERE emails.pk = $1;",
        )
        .bind(pk)
        .fetch_one(tx)
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))
    }

    pub async fn create_primary_active(
        tx: &mut SqliteConnection,
        user: User,
        email: String,
    ) -> Result<Self, AppError> {
        let activated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;

        Self::create_primary(tx, user, email, Some(activated_at)).await
    }

    pub async fn create_primary(
        tx: &mut SqliteConnection,
        user: User,
        email: String,
        activated_at: Option<i64>,
    ) -> Result<Self, AppError> {
        let pk = sqlx::query("INSERT INTO emails (is_primary, user_pk, activated_at, email) VALUES ($1, $2, $3, $4);")
            .bind(1)
            .bind(user.pk)
            .bind(activated_at)
            .bind(&email)
            .execute(tx)
            .await
            .map(|q| q.last_insert_rowid())?;
        Ok(Self { pk, user, email })
    }

    pub async fn create(
        tx: &mut SqliteConnection,
        is_primary: bool,
        user: User,
        email: String,
        activated_at: Option<i64>,
    ) -> Result<Self, AppError> {
        let pk = sqlx::query("INSERT INTO emails (is_primary, user_pk, activated_at, email) VALUES ($1, $2, $3, $4);")
            .bind(is_primary)
            .bind(user.pk)
            .bind(activated_at)
            .bind(&email)
            .execute(tx)
            .await
            .map(|q| q.last_insert_rowid())?;
        Ok(Self { pk, user, email })
    }

    pub async fn set_to_active(self, tx: &mut SqliteConnection) -> Result<Self, AppError> {
        let activated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        sqlx::query("UPDATE emails SET activated_at = $1 WHERE pk = $2;")
            .bind(activated_at)
            .bind(self.pk)
            .execute(tx)
            .await
            .map(|q| q.last_insert_rowid())?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::TestDatabase;

    use super::*;

    #[tokio::test]
    async fn test_email_account_get_by_email() {
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
            .unwrap();
        let email_account = EmailAccount::create_primary_active(
            &mut tx,
            user,
            "test_email_account_get_by_email@example.com".into(),
        )
        .await;
        tx.commit().await.unwrap();
        let account = email_account.unwrap();

        let result = EmailAccount::get_by_email(
            database.database(),
            "test_email_account_get_by_email@example.com",
        )
        .await
        .unwrap();

        assert!(result.is_some());
        let result = result.unwrap();

        assert_eq!(result.user.groups.0, vec![Group::Admin, Group::User]);

        sqlx::query("DELETE FROM users WHERE pk = $1;")
            .bind(account.user.pk)
            .execute(database.get_connection())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_find_by_email_with_password() {
        let database = TestDatabase::setup().await;

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut tx)
            .await
            .unwrap()
            .add_to_group(Group::User, &mut tx)
            .await
            .unwrap();
        let account = EmailAccount::create_primary_active(
            &mut tx,
            user,
            "test_find_by_email_with_password@example.com".into(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let result = User::find_by_email_with_password(
            database.database(),
            "test_find_by_email_with_password@example.com",
        )
        .await
        .unwrap();
        assert!(result.is_some());

        let result = result.unwrap();

        assert_eq!(result.password, "SDFdso34$hl#sdfj");
        assert_eq!(result.user.groups.0, vec![Group::User]);

        sqlx::query("DELETE FROM users WHERE pk = $1;")
            .bind(account.user.pk)
            .execute(database.get_connection())
            .await
            .unwrap();
    }

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
        assert_eq!(user.groups.0, vec![Group::Admin]);
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
        assert_eq!(user.groups.0, vec![Group::Admin, Group::User]);
    }

    #[tokio::test]
    async fn test_create_email_account() {
        let database = TestDatabase::setup().await;

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut tx).await.unwrap();
        let email_account = EmailAccount::create_primary_active(
            &mut tx,
            user,
            "test_create_email_account@example.com".into(),
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
