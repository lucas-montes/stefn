use crate::{database::Database, errors::AppError, sessions::Session, state::WebsiteState};

use super::{infrastructure::GoogleUserInfo, oauth::OauthTokenResponse};

pub trait GoogleOauthCallbackHook {
    type User: Send;

    fn find_user(
        user_info: &GoogleUserInfo,
        database: &Database,
    ) -> impl std::future::Future<Output = Result<Option<Self::User>, AppError>> + Send;

    fn create_user(
        state: &WebsiteState,
        token_response: &OauthTokenResponse,
        user_info: GoogleUserInfo,
    ) -> impl std::future::Future<Output = Result<Self::User, AppError>> + Send;

    fn user_found_hook(
        database: &Database,
        token_response: &OauthTokenResponse,
        user: Self::User,
    ) -> impl std::future::Future<Output = Result<Self::User, AppError>> + Send;

    fn run(
        token_response: &OauthTokenResponse,
        session: Session,
        state: &WebsiteState,
    ) -> impl std::future::Future<Output = Result<(), AppError>> + Send {
        async {
            let access_token = token_response.access_token();
            let user_info = GoogleUserInfo::get(access_token).await?.validate_email()?;
            let database = state.database();

            let user = match Self::find_user(&user_info, database).await? {
                Some(user) => Self::user_found_hook(database, token_response, user).await?,
                None => Self::create_user(state, token_response, user_info).await?,
            };

            Self::update_session(state, session, user).await
        }
    }

    fn update_session(
        state: &WebsiteState,
        session: Session,
        user: Self::User,
    ) -> impl std::future::Future<Output = Result<(), AppError>> + Send;
}
