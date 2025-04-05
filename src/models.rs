use std::{
    error::Error,
    fmt,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, prelude::FromRow, sqlite::SqliteRow, Decode, PgExecutor, Row, Type};

use crate::{database::Database, errors::AppError, log_and_wrap_custom_internal};

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

impl FromRow<'_, PgRow> for Groups {
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
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
    pub fn push(&mut self, group: Group) {
        self.0.push(group);
    }

    pub fn contains(&self, group: &Group) -> bool {
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
    pub fn new_authenticated(user: User) -> Self {
        Self(Some(user))
    }

    pub fn new_anonymous() -> Self {
        Self(None)
    }

    pub fn as_ref(&self) -> Option<&User> {
        self.0.as_ref()
    }
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
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE pk = $1 AND activated_at IS NOT NULL AND deactivated_at IS NULL);",
        )
        .bind(self.pk)
        .fetch_one(&**database)
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))
    }

    pub async fn create<'e, E: PgExecutor<'e>>(
        password: &str,
        activated_at: Option<NaiveDateTime>,
        executor: E,
    ) -> Result<Self, AppError> {
        let pk = sqlx::query_scalar(
            "INSERT INTO users (password, activated_at) VALUES ($1, $2) RETURNING pk;",
        )
        .bind(password)
        .bind(activated_at)
        .fetch_one(executor)
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))?;
        Ok(Self {
            pk,
            groups: Groups::default(),
        })
    }

    pub async fn create_active_default<'e, E: PgExecutor<'e>>(
        executor: E,
    ) -> Result<Self, AppError> {
        //TODO: hash this
        Self::create_active("SDFddg186DFG&$dfg987qzXZCDf688sf4so34$hl#sdfj", executor).await
    }

    pub async fn create_active<'e, E: PgExecutor<'e>>(
        password: &str,
        executor: E,
    ) -> Result<Self, AppError> {
        let activated_at = chrono::Utc::now().naive_utc();
        Self::create(password, Some(activated_at), executor).await
    }

    pub async fn add_to_group<'e, E: PgExecutor<'e>>(
        mut self,
        group: Group,
        executor: E,
    ) -> Result<Self, AppError> {
        if self.groups.contains(&group) {
            return Ok(self);
        }

        sqlx::query("INSERT INTO users_groups_m2m (user_pk, group_pk) VALUES ($1, $2);")
            .bind(self.pk)
            .bind(group as i64)
            .execute(executor)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        self.groups.push(group);
        Ok(self)
    }

    pub async fn set_to_active<'e, E: PgExecutor<'e>>(&self, executor: E) -> Result<u64, AppError> {
        let activated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        sqlx::query("UPDATE users SET activated_at = $1 WHERE pk = $2;")
            .bind(activated_at)
            .bind(self.pk)
            .execute(executor)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
            .map(|q| q.rows_affected())
    }

    pub async fn find_by_email_with_password(
        email: &str,
        database: &Database,
    ) -> Result<Option<UserWithPassword>, AppError> {
        sqlx::query_as(
            "SELECT emails.user_pk, STRING_AGG(users_groups_m2m.group_pk::TEXT, ',') as groups, users.password
                FROM emails
                INNER JOIN users ON users.pk = emails.user_pk
                LEFT JOIN users_groups_m2m ON emails.user_pk = users_groups_m2m.user_pk
                WHERE emails.email = $1
                GROUP BY emails.pk, users.password;",
        )
        .bind(email)
        .fetch_optional(&**database)
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
        self.email
            .split_once("@")
            .expect("The email address has no domain, did you forget the @?")
            .1
    }

    pub fn username(&self) -> &str {
        self.email
            .split_once("@")
            .expect("The email address has no username, did you forget the @?")
            .0
    }

    pub async fn get_by_pk<'e, E: PgExecutor<'e>>(pk: i64, executor: E) -> Result<Self, AppError> {
        sqlx::query_as(
            "
            SELECT emails.pk, emails.user_pk, emails.email, COALESCE(STRING_AGG(users_groups_m2m.group_pk::TEXT, ','), '') AS groups
            FROM emails
            LEFT JOIN users_groups_m2m ON users_groups_m2m.user_pk = emails.user_pk
            WHERE emails.pk = $1
            GROUP BY emails.pk, emails.user_pk, emails.email;",
        )
        .bind(pk)
        .fetch_one(executor)
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))
    }

    pub async fn create_primary_active<'e, E: PgExecutor<'e>>(
        user: User,
        email: String,
        executor: E,
    ) -> Result<Self, AppError> {
        let activated_at = chrono::Utc::now().naive_utc();

        Self::create_primary(user, email, Some(activated_at), executor).await
    }

    pub async fn create_primary<'e, E: PgExecutor<'e>>(
        user: User,
        email: String,
        activated_at: Option<NaiveDateTime>,
        executor: E,
    ) -> Result<Self, AppError> {
        Self::create(true, user, email, activated_at, executor).await
    }

    pub async fn create<'e, E: PgExecutor<'e>>(
        is_primary: bool,
        user: User,
        email: String,
        activated_at: Option<NaiveDateTime>,
        executor: E,
    ) -> Result<Self, AppError> {
        let pk = sqlx::query_scalar("INSERT INTO emails (is_primary, user_pk, activated_at, email) VALUES ($1, $2, $3, $4) RETURNING pk;")
            .bind(is_primary)
            .bind(user.pk)
            .bind(activated_at)
            .bind(&email)
            .fetch_one(executor)
            .await?;
        Ok(Self { pk, user, email })
    }

    pub async fn set_to_active<'e, E: PgExecutor<'e>>(self, executor: E) -> Result<Self, AppError> {
        let activated_at = chrono::Utc::now().naive_utc();

        sqlx::query("UPDATE emails SET activated_at = $1 WHERE pk = $2;")
            .bind(activated_at)
            .bind(self.pk)
            .execute(executor)
            .await?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use crate::database::Database;

    use super::*;

    #[sqlx::test(migrations = "./migrations/principal")]
    async fn test_find_by_email_with_password(pool: PgPool) {
        let database: Database = pool.into();
        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut *tx)
            .await
            .unwrap()
            .add_to_group(Group::User, &mut *tx)
            .await
            .unwrap();
        EmailAccount::create_primary_active(
            user,
            "test_find_by_email_with_password@example.com".into(),
            &mut *tx,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let result = User::find_by_email_with_password(
            "test_find_by_email_with_password@example.com",
            &database,
        )
        .await
        .unwrap();
        assert!(result.is_some());

        let result = result.unwrap();

        assert_eq!(result.password, "SDFddg186DFG&$dfg987qzXZCDf688sf4so34$hl#sdfj");
        assert_eq!(result.user.groups.0, vec![Group::User]);
    }

    #[sqlx::test(migrations = "./migrations/principal")]
    async fn test_create_active_default(pool: PgPool) {
        let database: Database = pool.into();

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut *tx).await;
        tx.commit().await.unwrap();

        assert!(user.is_ok());
        let user_pk = user.unwrap().pk;
        assert!(user_pk > 0);
    }

    #[sqlx::test(migrations = "./migrations/principal")]
    async fn test_create_admin_user(pool: PgPool) {
        let database: Database = pool.into();

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut *tx).await;

        assert!(user.is_ok());
        let user = user.unwrap().add_to_group(Group::Admin, &mut *tx).await;
        tx.commit().await.unwrap();
        assert!(user.is_ok());
        let user = user.unwrap();
        assert!(user.pk > 0);
        assert_eq!(user.groups.0, vec![Group::Admin]);
    }

    #[sqlx::test(migrations = "./migrations/principal")]
    async fn test_create_user_multiple_groups(pool: PgPool) {
        let database: Database = pool.into();

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut *tx)
            .await
            .unwrap()
            .add_to_group(Group::Admin, &mut *tx)
            .await
            .unwrap()
            .add_to_group(Group::User, &mut *tx)
            .await
            .unwrap()
            .add_to_group(Group::Admin, &mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert!(user.pk > 0);
        assert_eq!(user.groups.0, vec![Group::Admin, Group::User]);
    }

    #[sqlx::test(migrations = "./migrations/principal")]
    async fn test_create_email_account(pool: PgPool) {
        let database: Database = pool.into();

        let mut tx = database.start_transaction().await.unwrap();
        let user = User::create_active_default(&mut *tx).await.unwrap();
        let email_account = EmailAccount::create_primary_active(
            user,
            "test_create_email_account@example.com".into(),
            &mut *tx,
        )
        .await;
        tx.commit().await.unwrap();

        assert!(email_account.is_ok());
        let account = email_account.unwrap();
        assert!(account.pk > 0);

        let result: Option<(i64, i64, String)> =
            sqlx::query_as("SELECT pk, user_pk, email FROM emails WHERE pk = $1;")
                .bind(account.pk)
                .fetch_optional(&*database)
                .await
                .unwrap();

        assert!(result.is_some());
        let (db_pk, db_user_pk, db_email) = result.unwrap();
        assert_eq!(db_pk, account.pk);
        assert_eq!(db_user_pk, account.user.pk);
        assert_eq!(db_email, "test_create_email_account@example.com");
    }
}
