CREATE TABLE IF NOT EXISTS users (
    pk INTEGER PRIMARY KEY,
    password TEXT NOT NULL,
    activated_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS emails (
    pk INTEGER PRIMARY KEY,
    is_primary INTEGER NOT NULL DEFAULT 0,
    user_pk INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    activated_at INTEGER,
    email TEXT NOT NULL UNIQUE,
    CONSTRAINT fk_user
      FOREIGN KEY (user_pk) REFERENCES users (pk)
      ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_emails ON emails(email);

CREATE TABLE IF NOT EXISTS email_validations (
    slug TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    email_pk INTEGER NOT NULL,
    CONSTRAINT fk_email
      FOREIGN KEY (email_pk) REFERENCES emails (pk)
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS groups (
    pk INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS users_groups_m2m (
    user_pk INTEGER NOT NULL,
    group_pk INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    PRIMARY KEY (user_pk, group_pk),
    CONSTRAINT fk_user
      FOREIGN KEY (user_pk) REFERENCES users (pk)
      ON DELETE CASCADE,
    CONSTRAINT fk_group
      FOREIGN KEY (group_pk) REFERENCES groups (pk)
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS google_oauth_state (
    csrf_state TEXT NOT NULL,
    pkce_code_verifier TEXT NOT NULL,
    return_url TEXT NOT NULL
);

INSERT INTO groups (pk, created_at, name) VALUES 
(1, 1693440000, 'Admin'),
(2, 1693526400, 'User');