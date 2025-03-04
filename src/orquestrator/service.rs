use tokio::task::JoinSet;

use crate::{
    config::SharedConfig,
    service::{Service, ServiceExt},
    state::SharedState,
};

use super::tracing::init_tracing;

#[derive(Default)]
pub struct ServicesOrquestrator {
    config: SharedConfig,
    services: Vec<Service>,
    run_migrations: bool,
}

impl ServicesOrquestrator {
    pub fn set_config_from_env(mut self) -> Self {
        self.config = SharedConfig::from_env();
        self
    }

    pub fn enable_migrations(mut self) -> Self {
        self.run_migrations = true;
        self
    }

    pub fn init_tracing(self) -> Self {
        init_tracing(&self.config.env, self.config.sentry_token());
        self
    }

    pub fn add_service(mut self, service: Service) -> Self {
        self.services.push(service);
        self
    }

    async fn start_services(self) -> Vec<Result<(), std::io::Error>> {
        let mut set = JoinSet::new();

        let state = SharedState::new(&self.config);

        if self.run_migrations {
            state.database().run_migrations().await;
            state.events_broker().run_migrations().await;
        }

        for mut service in self.services {
            service.set_up(state.clone()).await;

            set.spawn(service.run());
        }

        set.join_all().await
    }

    pub fn run(self) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(self.config.worker_threads)
            .max_blocking_threads(self.config.max_blocking_threads)
            .build()
            .unwrap()
            .block_on(async { self.start_services().await });
    }
}
