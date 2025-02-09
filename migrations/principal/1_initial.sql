CREATE TABLE IF NOT EXISTS users (
    pk bigserial PRIMARY KEY,
    password TEXT NOT NULL,
    activated_at TIMESTAMP, 
    deactivated_at TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);


CREATE TABLE IF NOT EXISTS emails (
    pk bigserial PRIMARY KEY,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    user_pk BIGINT NOT NULL,
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    activated_at TIMESTAMP,
    email VARCHAR(255) NOT NULL UNIQUE,
    CONSTRAINT fk_user FOREIGN KEY (user_pk) REFERENCES users (pk) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_emails ON emails(email);

CREATE TABLE IF NOT EXISTS email_validations (
    slug VARCHAR(255) NOT NULL UNIQUE,
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    email_pk BIGINT NOT NULL,
    CONSTRAINT fk_email FOREIGN KEY (email_pk) REFERENCES emails (pk) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS groups (
    pk bigserial PRIMARY KEY,
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    name VARCHAR(255) NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS users_groups_m2m (
    user_pk BIGINT NOT NULL,
    group_pk BIGINT NOT NULL,
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_pk, group_pk),
    CONSTRAINT fk_user FOREIGN KEY (user_pk) REFERENCES users (pk) ON DELETE CASCADE,
    CONSTRAINT fk_group FOREIGN KEY (group_pk) REFERENCES groups (pk) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS google_oauth_state (
    csrf_state VARCHAR(255) NOT NULL,
    pkce_code_verifier VARCHAR(255) NOT NULL,
    return_url VARCHAR(255) NOT NULL
);

INSERT INTO groups (pk, created_at, name) VALUES 
(1, CURRENT_TIMESTAMP, 'Admin'),
(2, CURRENT_TIMESTAMP, 'User');

CREATE TABLE IF NOT EXISTS profiles (
    pk bigserial PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    given_name VARCHAR(255) NOT NULL,
    family_name VARCHAR(255) NOT NULL,
    picture TEXT NOT NULL,
    parsing_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    quota BIGINT NOT NULL DEFAULT 0 CHECK (quota >= 0),
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_pk BIGINT NOT NULL,
    CONSTRAINT fk_user FOREIGN KEY (user_pk) REFERENCES users (pk) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS google_profiles (
    pk bigserial PRIMARY KEY,
    id VARCHAR(255) NOT NULL UNIQUE,
    email_pk BIGINT NOT NULL,
    access_token VARCHAR(255) NOT NULL,
    refresh_token VARCHAR(255) NOT NULL,
    expires_in BIGINT NOT NULL,
    updated_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_email FOREIGN KEY (email_pk) REFERENCES emails (pk) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_google_profiles_id ON google_profiles(id);
