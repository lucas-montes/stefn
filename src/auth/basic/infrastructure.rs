use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::{database::Database, log_and_wrap_custom_internal, service::AppError};

#[derive(Debug)]
pub struct EmailValidationManager {
    pub email_pk: i64,
    pub slug: String,
}

impl EmailValidationManager {
    pub fn new(email_pk: i64) -> Self {
        Self {
            email_pk,
            slug: Uuid::new_v4().to_string(),
        }
    }

    pub async fn delete_and_get_email_pk(
        tx: &mut SqliteConnection,
        slug: String,
    ) -> Result<Self, AppError> {
        let email_pk = sqlx::query_as::<_, (i64,)>(
            "DELETE FROM email_validations WHERE slug = $1 RETURNING email_pk;",
        )
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))?
        .0;

        Ok(Self { email_pk, slug })
    }

    pub async fn send(self) -> Result<Self, AppError> {
        Ok(self)
    }

    pub async fn save(self, database: &Database) -> Result<Self, AppError> {
        sqlx::query("INSERT INTO email_validations (email_pk, slug) VALUES ($1, $2);")
            .bind(self.email_pk)
            .bind(&self.slug)
            .execute(database.get_connection())
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        //TODO: if error because not unique cretreade self
        Ok(self)
    }
}
