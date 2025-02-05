use axum::async_trait;
use sqlx::{sqlite::SqliteRow, QueryBuilder};

use crate::{log_and_wrap_custom_internal, service::AppError};

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
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        let mut query_builder = QueryBuilder::new("SELECT * FROM ");
        query_builder.push(Self::TABLE);
        if let Where::Pk(pk) = clause {
            query_builder.push(" WHERE pk = $1;");
            query_builder
                .build_query_as()
                .bind(pk)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| log_and_wrap_custom_internal!(e))
        } else {
            query_builder.push(" WHERE pk = $1;");
            query_builder
                .build_query_as()
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| log_and_wrap_custom_internal!(e))
        }
    }

    async fn delete_by(database: &Database, clause: Where) -> Result<(), AppError> {
        let mut tx = database
            .get_connection()
            .begin()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        if let Where::Pk(pk) = clause {
            sqlx::query("DELETE FROM $1 WHERE pk = $2;")
                .bind(Self::TABLE)
                .bind(pk)
                .execute(&mut *tx)
                .await
                .map_err(|e| log_and_wrap_custom_internal!(e))?;
        }
        Ok(())
    }

    async fn filter(database: &Database, clause: Where) -> Result<(), AppError> {
        let mut tx = database
            .get_connection()
            .begin()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        if let Where::Pk(pk) = clause {
            sqlx::query("DELETE FROM $1 WHERE pk = $2;")
                .bind(Self::TABLE)
                .bind(pk)
                .execute(&mut *tx)
                .await
                .map_err(|e| log_and_wrap_custom_internal!(e))?;
        }
        Ok(())
    }

    async fn create(database: &Database) {
        //TODO: finish
        let fields = [""].join(",");
        let values = (1..fields.len())
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
            .join(",");

        let q = format!(
            "INSERT INTO {table} ({fields}) VALUES ({values});",
            table = Self::TABLE
        );
        sqlx::query(&q)
            .execute(database.get_connection())
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
            .unwrap();
    }
}
