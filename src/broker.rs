use chrono::{NaiveDateTime, Utc};
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    QueryBuilder, SqlitePool,
};
use std::ops::Deref;

use crate::errors::AppError;

#[derive(Clone, Debug)]
pub struct Broker(SqlitePool);

//TODO: maybe, just maybe, i should use the primary database for that

impl Deref for Broker {
    type Target = SqlitePool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Broker {
    pub fn new(broker_url: &str) -> Self {
        let events_config = SqliteConnectOptions::new()
            .filename(broker_url)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .create_if_missing(true);

        Self(SqlitePool::connect_lazy_with(events_config))
    }

    pub async fn run_migrations(&self) {
        Migrator::new(std::path::Path::new("./migrations/events"))
            .await
            .expect("Where are the migrations?")
            .run(&**self)
            .await
            .expect("Migrations failed");
    }

    pub async fn send_events<S, C>(&self, events: EventFactory<S, C>) -> Result<u64, AppError>
    where
        C: Clone + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'static, sqlx::Sqlite> + 'static,
        S: Clone + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'static, sqlx::Sqlite> + 'static,
    {
        self.insert_events(events).await
    }

    ///Return the number of events inserted
    async fn insert_events<S, C>(&self, events: EventFactory<S, C>) -> Result<u64, AppError>
    where
        C: Clone + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'static, sqlx::Sqlite> + 'static,
        S: Clone + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'static, sqlx::Sqlite> + 'static,
    {
        let mut tx = self.begin().await?;

        let mut query_builder = QueryBuilder::new("INSERT INTO ");
        query_builder
            .push(events.table())
            .push(" (source, command, version, priority, created_at, payload) ")
            .push_values(events, |mut b, event| {
                b.push_bind(event.metadata.source)
                    .push_bind(event.metadata.command)
                    .push_bind(event.metadata.version)
                    .push_bind(event.priority)
                    .push_bind(event.created_at)
                    .push_bind(event.payload);
            });
        //TODO: should I add a ; ?

        let result = query_builder
            .build()
            .execute(&mut *tx)
            .await?
            .rows_affected();

        tx.commit().await?;
        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct EventMetadata<S, C>
where
    C: Clone,
    S: Clone,
{
    source: S,
    command: C,
    table: String,
    version: String,
}

impl<S, C> EventMetadata<S, C>
where
    C: Clone,
    S: Clone,
{
    pub fn new(source: S, command: C, table: String, version: String) -> Self {
        Self {
            source,
            command,
            table,
            version,
        }
    }
}

//TODO: do we want to clone that much?
#[derive(Debug)]
pub struct Event<S, C>
where
    C: Clone,
    S: Clone,
{
    metadata: EventMetadata<S, C>,
    created_at: NaiveDateTime,
    priority: u8,
    payload: Vec<u8>,
}

pub struct EventFactory<S, C>
where
    C: Clone,
    S: Clone,
{
    metadata: EventMetadata<S, C>,
    data: std::vec::IntoIter<Vec<u8>>,
}

impl<S, C> EventFactory<S, C>
where
    C: Clone,
    S: Clone,
{
    pub fn new(metadata: EventMetadata<S, C>, data: Vec<Vec<u8>>) -> Self {
        Self {
            metadata,
            data: data.into_iter(),
        }
    }

    pub fn table(&self) -> &str {
        &self.metadata.table
    }

    fn new_message(&self, payload: Vec<u8>) -> Event<S, C> {
        Event {
            metadata: self.metadata.clone(),
            created_at: Utc::now().naive_utc(),
            priority: 0,
            payload,
        }
    }
}

impl<S, C> Iterator for EventFactory<S, C>
where
    C: Clone,
    S: Clone,
{
    type Item = Event<S, C>;

    fn next(&mut self) -> Option<Self::Item> {
        self.data.next().map(|p| self.new_message(p))
    }
}
