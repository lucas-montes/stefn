use crate::{database::Database, service::AppError};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub pk: i64,
    pub password: String,
    pub groups: String,
}
//TODO: why two users

pub async fn find_user_by_email(
    database: &Database,
    email: &str,
) -> Result<Option<User>, AppError> {
    sqlx::query_as(
        r#"
            SELECT emails.user_pk as pk, users.password, GROUP_CONCAT(groups.name, ', ') AS groups
            FROM emails
            INNER JOIN users ON emails.user_pk = users.pk
            LEFT JOIN users_groups_m2m ON emails.user_pk = users_groups_m2m.user_pk
            LEFT JOIN groups ON users_groups_m2m.group_pk = groups.pk
            WHERE emails.email = $1
            HAVING count(emails.user_pk) > 0;
        "#,
    )
    .bind(email)
    .fetch_optional(database.get_connection())
    .await
    .map_err(|e| AppError::custom_internal(&e.to_string()))
}
