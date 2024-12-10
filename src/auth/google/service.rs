use axum::async_trait;

use crate::{
    database::Database,
    models::{EmailAccount, Group, User},
    service::AppError,
    sessions::{Session, Sessions},
    state::WebsiteState,
};

use super::{infrastructure::GoogleUserInfo, oauth::OauthTokenResponse};

#[async_trait]
pub trait GoogleOauthCallbackHook {
    type User: Send;

    async fn find_user(
        user_info: &GoogleUserInfo,
        database: &Database,
    ) -> Result<Option<Self::User>, AppError>;

    async fn create_user(
        database: &Database,
        token_response: &OauthTokenResponse,
        user_info: GoogleUserInfo,
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
            Some(user) => user,
            None => Self::create_user(database, token_response, user_info).await?,
        };

        Self::update_session(state.sessions(), session, &user).await
    }

    async fn update_session(
        sessions: &Sessions,
        session: Session,
        user: &Self::User,
    ) -> Result<(), AppError>;
}

// pub trait GoogleOauthCallbackHook {
//     async fn find_user(
//         user_info: &GoogleUserInfo,
//         database: &Database,
//     ) -> Result<Option<EmailAccount>, AppError> {
//         EmailAccount::get_by_email(database, &user_info.email).await
//     }

//     async fn create_user(
//         user_info: GoogleUserInfo,
//         database: &Database,
//     ) -> Result<EmailAccount, AppError> {
//         let mut tx = database.start_transaction().await?;

//         let user = User::create_active_default(&mut tx)
//             .await?
//             .add_to_group(Group::User, &mut tx)
//             .await?;

//         user.add_profile(
//             &mut tx,
//             &user_info.name,
//             &user_info.given_name,
//             &user_info.family_name,
//             &user_info.picture,
//         )
//         .await?;
//         let email_account =
//             EmailAccount::create_primary_active(&mut tx, user, &user_info.email).await?;

//         tx.commit()
//             .await
//             .map_err(|e| AppError::custom_internal(&e.to_string()))?;
//         Ok(email_account)
//     }

// }
