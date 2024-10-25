use menva::read_default_file;
use tokio::task::JoinSet;

use crate::service::{Service, Services};

use super::tracing::{init_dev_tracing, init_prod_tracing};

pub struct ServicesOrquestrator {
    max_blocking_threads: usize,
    worker_threads: usize,
    services: Vec<Services>,
    run_migrations: bool,
}

impl ServicesOrquestrator {
    pub fn new(worker_threads: usize, max_blocking_threads: usize) -> Self {
        Self {
            worker_threads,
            max_blocking_threads,
            services: Vec::new(),
            run_migrations: false,
        }
    }
    pub fn load_environment_variables(self) -> Self {
        read_default_file();
        self
    }

    pub fn run_migrations(mut self) -> Self {
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

    pub fn add_service(mut self, service: Services) -> Self {
        self.services.push(service);
        self
    }

    async fn start_services(self) -> Vec<Result<(), std::io::Error>> {
        let mut set = JoinSet::new();

        for mut service in self.services {
            service.set_up();
            if self.run_migrations {
                service.run_migrations().await;
            }
            set.spawn(async move { service.run().await });
        }

        set.join_all().await
    }

    pub fn run(self) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(self.worker_threads)
            .max_blocking_threads(self.max_blocking_threads)
            .build()
            .unwrap()
            .block_on(async { self.start_services().await });
    }
}
