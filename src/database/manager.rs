use axum::async_trait;
use sqlx::{sqlite::SqliteRow, QueryBuilder};

use crate::AppError;

use super::Database;

pub enum Where<T: Send = ()> {
    Pk(i64),
    Condition(T),
}

impl Where {
    pub fn from_pk(pk: i64) -> Self {
        Self::Pk(pk)
    }
}

#[async_trait]
pub trait Manager {
    const TABLE: &str;

    async fn get_by<M: Send + Unpin + for<'r> sqlx::FromRow<'r, SqliteRow>>(
        database: &Database,
        clause: Where,
    ) -> Result<Option<M>, AppError> {
        let mut tx = database
            .get_connection()
            .begin()
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))?;

        let mut query_builder = QueryBuilder::new("SELECT * FROM ");
        query_builder.push(Self::TABLE);
        if let Where::Pk(pk) = clause {
            query_builder.push(" WHERE pk = $1;");
            query_builder
                .build_query_as()
                .bind(pk)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::custom_internal(&e.to_string()))
        } else {
            query_builder.push(" WHERE pk = $1;");
            query_builder
                .build_query_as()
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::custom_internal(&e.to_string()))
        }
    }

    async fn delete_by(database: &Database, clause: Where) -> Result<(), AppError> {
        let mut tx = database
            .get_connection()
            .begin()
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))?;
        if let Where::Pk(pk) = clause {
            sqlx::query("DELETE FROM $1 WHERE pk = $2;")
                .bind(Self::TABLE)
                .bind(pk)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::custom_internal(&e.to_string()))?;
        }
        Ok(())
    }

    async fn filter(database: &Database, clause: Where) -> Result<(), AppError> {
        let mut tx = database
            .get_connection()
            .begin()
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))?;
        if let Where::Pk(pk) = clause {
            sqlx::query("DELETE FROM $1 WHERE pk = $2;")
                .bind(Self::TABLE)
                .bind(pk)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::custom_internal(&e.to_string()))?;
        }
        Ok(())
    }

    async fn create(database: &Database) {}
}
