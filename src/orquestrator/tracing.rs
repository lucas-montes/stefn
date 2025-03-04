use sentry_tracing::EventFilter;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Env;
//TODO: refactor. Create a better logging per service and global
pub fn init_tracing(env: &Env, sentry_token: Option<&str>) {
    let registry = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                // .with_writer(non_blocking)
                .log_internal_errors(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_current_span(true)
                .with_span_events(FmtSpan::FULL)
                .with_span_list(true)
                .with_target(true),
        );

    if let Some(token) = sentry_token {
        let debug = if let Env::Development | Env::Test = env {
            true
        } else {
            false
        };
        let _guard = sentry::init((
            token,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                environment: Some(env.to_string().into()),
                debug,
                ..Default::default()
            },
        ));

        let sentry_layer = sentry_tracing::layer().event_filter(|md| match md.level() {
            &tracing::Level::WARN => EventFilter::Event,
            _ => EventFilter::Ignore,
        });

        registry.with(sentry_layer).init();
    } else {
        registry.init();
    };
}
