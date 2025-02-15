use lettre::Message;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    config::{ServiceConfig, WebsiteConfig},
    database::Database,
    log_and_wrap_custom_internal,
    mailing::Mailer,
    errors::AppError,
};

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
        slug: String,
        tx: &mut PgConnection,
    ) -> Result<Self, AppError> {
        let email_pk =
            sqlx::query_scalar("DELETE FROM email_validations WHERE slug = $1 RETURNING email_pk;")
                .bind(&slug)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| log_and_wrap_custom_internal!(e))?;

        Ok(Self { email_pk, slug })
    }

    pub async fn send(
        self,
        config: &WebsiteConfig,
        mailer: &Mailer,
        to: &str,
    ) -> Result<Self, AppError> {
        let body = format!(
            "Please click the following link to validate your email: {}",
            config.build_url(&format!("/email-validation/{}", self.slug))
        );
        let message = Message::builder()
            .from(config.email_default_sender.parse().unwrap())
            .to(to.parse().unwrap())
            .subject("Welcome to Smartlink")
            .body(body)
            .expect("failed to build email");
        mailer.send(&message).await?;
        Ok(self)
    }

    pub async fn save(self, database: &Database) -> Result<Self, AppError> {
        sqlx::query("INSERT INTO email_validations (email_pk, slug) VALUES ($1, $2);")
            .bind(self.email_pk)
            .bind(&self.slug)
            .execute(&**database)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        //TODO: check that the slug is unique
        Ok(self)
    }
}
