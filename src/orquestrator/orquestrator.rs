use std::sync::Arc;

use menva::read_default_file;
use tokio::task::JoinSet;

use crate::{
    config::SharedConfig,
    service::{Service, ServiceExt},
    state::SharedState,
};

use super::tracing::{init_dev_tracing, init_prod_tracing};

pub struct ServicesOrquestrator {
    config: Option<SharedConfig>,
    services: Vec<Service>,
    run_migrations: bool,
}

impl ServicesOrquestrator {
    pub fn new(config: SharedConfig) -> Self {
        Self {
            config: Some(config),
            services: Vec::new(),
            run_migrations: false,
        }
    }
    pub fn default() -> Self {
        Self {
            config: None,
            services: Vec::new(),
            run_migrations: false,
        }
    }

    pub fn load_environment_variables(self) -> Self {
        read_default_file();
        self
    }
    pub fn set_config_from_env(mut self) -> Self {
        self.config = Some(SharedConfig::from_env());
        self
    }

    pub fn enable_migrations(mut self) -> Self {
        self.run_migrations = true;
        self
    }

    pub fn init_dev_tracing(self) -> Self {
        init_dev_tracing();
        self
    }

    pub fn init_prod_tracing(self) -> Self {
        init_prod_tracing();
        self
    }

    pub fn add_service(mut self, service: Service) -> Self {
        self.services.push(service);
        self
    }

    async fn start_services(self) -> Vec<Result<(), std::io::Error>> {
        let mut set = JoinSet::new();

        let state = SharedState::new(
            self.config
                .as_ref()
                .expect("Missing the shared config in the orquestrator"),
        );

        if self.run_migrations {
            state.database().run_migrations().await;
            state.events_broker().run_migrations().await;
        }

        for mut service in self.services {
            service.set_up(state.clone()).await;

            set.spawn(async move { service.run().await });
        }

        set.join_all().await
    }

    pub fn run(self) {
        let config = self
            .config
            .as_ref()
            .expect("Missing the shared config in the orquestrator");
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(config.worker_threads)
            .max_blocking_threads(config.max_blocking_threads)
            .build()
            .unwrap()
            .block_on(async { self.start_services().await });
    }
}
