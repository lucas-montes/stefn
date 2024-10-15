use tokio::task::JoinSet;

use crate::service::Service;

use super::tracing::{init_dev_tracing, init_prod_tracing};

pub struct ServicesOrquestrator {
    max_blocking_threads: usize,
    worker_threads: usize,
    services: Vec<Service>,
}

impl ServicesOrquestrator {
    pub fn new(worker_threads: usize, max_blocking_threads: usize) -> Self {
        Self {
            worker_threads,
            max_blocking_threads,
            services: Vec::new(),
        }
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

    async fn start_services(&self) {
        let mut set = JoinSet::new();

        for service in self.services.clone() {
            set.spawn(async move { service.run().await });
        }

        let output = set.join_all().await;
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
