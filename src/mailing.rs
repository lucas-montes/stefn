use std::sync::Arc;

use lettre::{
    transport::smtp::{authentication::Credentials, client::Tls, response::Response},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::{config::SharedConfig, log_and_wrap_custom_internal, service::AppError};

#[derive(Clone, Debug)]
pub struct Mailer(Arc<AsyncSmtpTransport<Tokio1Executor>>);

impl Default for Mailer {
    fn default() -> Self {
        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay("0.0.0.0")
                .unwrap()
                .port(1025)
                .tls(Tls::None)
                .build();
        Self(Arc::new(mailer))
    }
}

impl Mailer {
    pub fn new(config: &SharedConfig) -> Self {
        let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_relay)
                .expect("Something went wrong with the smtp transport for the Mailer")
                .credentials(creds)
                .build();
        Self(Arc::new(mailer))
    }
    pub fn sutb() -> Self {
        Self::default()
    }
    pub async fn send(&self, message: &Message) -> Result<Response, AppError> {
        let raw = message.formatted();
        let envelope = message.envelope();
        self.0
            .send_raw(envelope, &raw)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
    }
}
