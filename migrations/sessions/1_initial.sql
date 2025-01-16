CREATE TABLE IF NOT EXISTS web_sessions (
    session_id TEXT PRIMARY KEY NOT NULL,
    user_pk INTEGER,
    groups TEXT,
    last_accessed INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    expiration INTEGER NOT NULL,
    data BLOB,
    country TEXT
);