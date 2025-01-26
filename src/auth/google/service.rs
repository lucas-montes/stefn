use axum::async_trait;

use crate::{database::Database, service::AppError, sessions::Session, state::WebsiteState};

use super::{infrastructure::GoogleUserInfo, oauth::OauthTokenResponse};

#[async_trait]
pub trait GoogleOauthCallbackHook {
    type User: Send;

    async fn find_user(
        user_info: &GoogleUserInfo,
        database: &Database,
    ) -> Result<Option<Self::User>, AppError>;

    async fn create_user(
        state: &WebsiteState,
        token_response: &OauthTokenResponse,
        user_info: GoogleUserInfo,
    ) -> Result<Self::User, AppError>;

    async fn user_found_hook(
        database: &Database,
        token_response: &OauthTokenResponse,
        user: Self::User,
    ) -> Result<Self::User, AppError>;

    async fn run(
        token_response: &OauthTokenResponse,
        session: Session,
        state: &WebsiteState,
    ) -> Result<(), AppError> {
        let access_token = token_response.access_token();
        let user_info = GoogleUserInfo::get(access_token).await?.validate_email()?;
        let database = state.database();

        let user = match Self::find_user(&user_info, database).await? {
            Some(user) => Self::user_found_hook(database, token_response, user).await?,
            None => Self::create_user(state, token_response, user_info).await?,
        };

        Self::update_session(state, session, user).await
    }

    async fn update_session(
        state: &WebsiteState,
        session: Session,
        user: Self::User,
    ) -> Result<(), AppError>;
}
