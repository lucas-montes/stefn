use serde::Deserialize;

use crate::{log_and_wrap_custom_internal, service::AppError};

#[derive(Deserialize)]
pub struct GoogleUserInfo {
    pub email: String,
    pub given_name: String,
    pub family_name: String,
    pub id: String,
    pub name: String,
    pub picture: String,
    verified_email: bool,
}

impl GoogleUserInfo {
    pub fn stub() -> Self {
        Self {
            email: "GoogleUserInfo@example.com".to_string(),
            given_name: "Test".to_string(),
            family_name: "User".to_string(),
            id: "google-id-666".to_string(),
            name: "Test User".to_string(),
            picture: "http://example.com/picture.jpg".to_string(),
            verified_email: true,
        }
    }

    pub async fn get(access_token: &str) -> Result<Self, AppError> {
        let url =
            "https://www.googleapis.com/oauth2/v2/userinfo?oauth_token=".to_owned() + access_token;
        reqwest::get(url)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))?
            .json()
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
    }

    pub fn validate_email(self) -> Result<Self, AppError> {
        self.verified_email
            .then_some(self)
            .ok_or(AppError::custom_bad_request(
                "You need to validate your email with google",
            ))
    }
}
