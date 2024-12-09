use crate::{
    database::Database,
    models::{EmailAccount, Group, User},
    service::AppError,
    sessions::Session,
    state::WebsiteState,
};

use super::{infrastructure::GoogleUserInfo, oauth::OauthTokenResponse};

pub trait GoogleOauthCallbackHook {
    async fn find_user(
        user_info: &GoogleUserInfo,
        database: &Database,
    ) -> Result<Option<EmailAccount>, AppError> {
        EmailAccount::find(database, &user_info.email).await
    }

    async fn create_user(
        user_info: GoogleUserInfo,
        database: &Database,
    ) -> Result<EmailAccount, AppError> {
        let mut tx = database.start_transaction().await?;

        let user = User::create_active(&mut tx)
            .await?
            .add_to_group(Group::User, &mut tx)
            .await?;

        user.add_profile(
            &mut tx,
            &user_info.name,
            &user_info.given_name,
            &user_info.family_name,
            &user_info.picture,
        )
        .await?;
        let email_account =
            EmailAccount::create_primary_active(&mut tx, user, &user_info.email).await?;

        tx.commit()
            .await
            .map_err(|e| AppError::custom_internal(&e.to_string()))?;
        Ok(email_account)
    }

    async fn run(
        token_response: &OauthTokenResponse,
        session: Session,
        state: &WebsiteState,
    ) -> Result<(), AppError> {
        let access_token = token_response.access_token();
        let user_info = GoogleUserInfo::get(access_token).await?.validate_email()?;
        let database = state.database();

        let email_account = match Self::find_user(&user_info, database).await? {
            Some(profile) => profile,
            None => Self::create_user(user_info, database).await?,
        };

        state
            .sessions()
            .reuse_current_as_new_one(session, email_account.user.pk, email_account.user.groups)
            .await
    }
}
