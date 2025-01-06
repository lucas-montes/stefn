use std::time::Duration;

use oauth2::{
    basic::{BasicClient, BasicTokenType},
    reqwest::async_http_client,
    AccessToken, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EmptyExtraTokenFields, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken,
    RevocationUrl, Scope, StandardTokenResponse, TokenResponse, TokenUrl,
};

use reqwest::Url;

use crate::{
    config::{WebsiteConfig, ServiceConfig}, database::Database, log_and_wrap_custom_internal, service::AppError,
};

#[derive(Debug)]
pub struct OauthTokenResponse(pub StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>);

impl OauthTokenResponse {
    pub fn stub() -> Self {
        let mut token = StandardTokenResponse::new(
            AccessToken::new("access_token".into()),
            BasicTokenType::Bearer,
            EmptyExtraTokenFields {},
        );
        token.set_refresh_token(Some(RefreshToken::new("refresh_token".into())));
        token.set_expires_in(Some(&Duration::new(3600, 0)));
        Self(token)
    }

    pub fn access_token(&self) -> &str {
        self.0.access_token().secret()
    }

    pub fn refresh_token(&self) -> Option<&String> {
        self.0.refresh_token().map(|t| t.secret())
    }

    pub fn expires_in(&self) -> Option<i64> {
        self.0.expires_in().map(|d| d.as_secs() as i64)
    }

    pub async fn login(
        config: &WebsiteConfig,
        code: String,
        pkce_code: String,
    ) -> Result<Self, AppError> {
        let client = get_client(
            config.build_url("/callback"),
            config.google_client_id.clone(),
            config.google_client_secret.clone(),
        )?;
        client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_code))
            .request_async(async_http_client)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
            .map(Self)
    }

    pub async fn new_token(
        config: &WebsiteConfig,
        refresh_token: String,
    ) -> Result<Self, AppError> {
        let client = get_client(
            config.build_url("/callback"),
            config.google_client_id.clone(),
            config.google_client_secret.clone(),
        )?;
        client
            .exchange_refresh_token(&RefreshToken::new(refresh_token))
            .request_async(async_http_client)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
            .map(Self)
    }
}

pub struct CallbackValidation {
    authorize_url: Url,
    pkce_code_verifier: PkceCodeVerifier,
    csrf_state: CsrfToken,
}

impl CallbackValidation {
    pub async fn validate(
        params_state: String,
        database: &Database,
    ) -> Result<(String, String), AppError> {
        let mut tx: sqlx::Transaction<'_, sqlx::Sqlite> = database.start_transaction().await?;

        let csrf_state = CsrfToken::new(params_state);

        let query: (String, String) = sqlx::query_as(
            r#"SELECT pkce_code_verifier, return_url FROM google_oauth_state WHERE csrf_state = $1;"#,
        )
        .bind(csrf_state.secret())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))?;
        let _ = sqlx::query("DELETE FROM google_oauth_state WHERE csrf_state = $1;")
            .bind(csrf_state.secret())
            .execute(&mut *tx)
            .await;

        tx.commit()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;
        Ok(query)
    }

    pub fn new(config: &WebsiteConfig, scopes: Vec<Scope>) -> Result<Self, AppError> {
        let client = get_client(
            config.build_url("/callback"),
            config.google_client_id.clone(),
            config.google_client_secret.clone(),
        )?;

        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();
        //TODO: make it more modular. Maybe we dont want to prompt each every time.
        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .add_extra_param("access_type", "offline")
            .add_extra_param("prompt", "consent")
            .add_scopes(scopes)
            .set_pkce_challenge(pkce_code_challenge)
            .url();
        Ok(Self {
            authorize_url,
            pkce_code_verifier,
            csrf_state,
        })
    }

    pub async fn save(self, database: &Database, return_url: &str) -> Result<Self, AppError> {
        let mut tx = database.start_transaction().await?;

        sqlx::query(
            "INSERT INTO google_oauth_state (csrf_state, pkce_code_verifier, return_url) VALUES ($1, $2, $3);",
        )
        .bind(self.csrf_state.secret())
        .bind(self.pkce_code_verifier.secret())
        .bind(return_url)
        .execute(&mut *tx)
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))?;

        tx.commit()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?;

        Ok(self)
    }

    pub fn authorize_url(self) -> Url {
        self.authorize_url
    }
}

//TODO: move this client to the state
pub fn get_client(
    redirect_url: String,
    client_id: String,
    client_secret: String,
) -> Result<BasicClient, AppError> {
    let google_client_id = ClientId::new(client_id);
    let google_client_secret = ClientSecret::new(client_secret);
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string())
        .map_err(|_| AppError::custom_internal("OAuth: invalid authorization endpoint URL"))?;
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
        .map_err(|_| AppError::custom_internal("OAuth: invalid token endpoint URL"))?;

    // Set up the config for the Google OAuth2 process.
    let client = BasicClient::new(
        google_client_id,
        Some(google_client_secret),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(
        RedirectUrl::new(redirect_url)
            .map_err(|_| AppError::custom_internal("OAuth: invalid redirect URL"))?,
    )
    .set_revocation_uri(
        RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string())
            .map_err(|_| AppError::custom_internal("OAuth: invalid revocation endpoint URL"))?,
    );
    Ok(client)
}
