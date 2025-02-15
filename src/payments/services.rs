use std::{fmt::Debug, ops::Deref, str::FromStr};

use stripe::{
    CheckoutSession, CheckoutSessionId, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CustomerId,
};

use crate::{log_and_wrap_custom_internal, errors::AppError};

#[derive(Clone)]
pub struct PaymentsProcessor(Client);

impl Deref for PaymentsProcessor {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for PaymentsProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaymentsProcessor").finish()
    }
}

impl PaymentsProcessor {
    pub fn new(stripe_private_key: &str) -> Self {
        Self(Client::new(stripe_private_key))
    }

    pub async fn retrieve_checkout_session(
        &self,
        session_id: &str,
    ) -> Result<stripe::CheckoutSession, AppError> {
        CheckoutSession::retrieve(
            &self.0,
            &CheckoutSessionId::from_str(session_id)
                .map_err(|e| log_and_wrap_custom_internal!(e))?,
            &[],
        )
        .await
        .map_err(|e| log_and_wrap_custom_internal!(e))
    }

    pub async fn create_checkout_session(
        &self,
        ui_mode: CheckoutSessionUiMode<'_>,
        mode: CheckoutSessionMode,
        customer: Option<String>,
        price: String,
    ) -> Result<stripe::CheckoutSession, AppError> {
        let mut params = CreateCheckoutSession::new();
        params.ui_mode = Some(stripe::CheckoutSessionUiMode::Embedded);
        match ui_mode {
            CheckoutSessionUiMode::Embedded(return_url) => {
                params.return_url = Some(return_url);
            }
            CheckoutSessionUiMode::Hosted(cancel_url, success_url) => {
                params.cancel_url = Some(cancel_url);
                params.success_url = Some(success_url);
            }
        }
        params.mode = Some(mode);
        if let Some(customer_id) = customer.as_ref() {
            params.customer = Some(
                CustomerId::from_str(customer_id).map_err(|e| log_and_wrap_custom_internal!(e))?,
            );
        };

        params.line_items = Some(vec![CreateCheckoutSessionLineItems {
            quantity: Some(1), //TODO: will this be always 1? no
            price: Some(price),
            ..Default::default()
        }]);

        stripe::CheckoutSession::create(&self.0, params)
            .await
            .map_err(|e| log_and_wrap_custom_internal!(e))
    }
}

pub enum CheckoutSessionUiMode<'a> {
    Embedded(&'a str),
    Hosted(&'a str, &'a str),
}
