use axum::Router;
use futures::future::BoxFuture;
use sqlx::migrate::Migrator;
use std::net::SocketAddr;
use tokio::{net::TcpListener, signal};

use super::{get_router, AppState, ServiceConfig};

pub enum Services {
    Background(BackgroundService),
    Http(HttpService),
}

pub struct BackgroundService {
    config: ServiceConfig,
    task: Box<dyn Fn() -> BoxFuture<'static, Result<(), std::io::Error>> + Send>,
}

impl BackgroundService {
    pub fn new(
        config_path: &str,
        task: fn() -> BoxFuture<'static, Result<(), std::io::Error>>,
    ) -> Self {
        Self {
            config: ServiceConfig::from_file(config_path),
            task: Box::new(task),
        }
    }
}

impl Service for BackgroundService {
    fn stub(self) -> Self {
        Self {
            config: ServiceConfig::stub(),
            task: self.task,
        }
    }

    async fn run(self) -> Result<(), std::io::Error> {
        (self.task)().await
    }
}

impl Services {
    pub fn new_http_service(
        config_path: &str,
        router_factory: fn(AppState) -> Router<AppState>,
    ) -> Self {
        Self::Http(HttpService::new(config_path, router_factory))
    }

    pub fn new_background_service(
        config_path: &str,
        task: fn() -> BoxFuture<'static, Result<(), std::io::Error>>,
    ) -> Self {
        Self::Background(BackgroundService::new(config_path, task))
    }

    pub fn router(&self) -> Option<Router> {
        match self {
            Self::Background(_) => None,
            Self::Http(s) => Some(s.router()),
        }
    }
}

impl Service for Services {
    fn stub(self) -> Self {
        match self {
            Self::Background(s) => Self::Background(s.stub()),
            Self::Http(s) => Self::Http(s.stub()),
        }
    }
    fn set_up(&mut self) {
        match self {
            Self::Background(s) => s.set_up(),
            Self::Http(s) => s.set_up(),
        }
    }
    async fn run(self) -> Result<(), std::io::Error> {
        match self {
            Self::Background(s) => s.run().await,
            Self::Http(s) => s.run().await,
        }
    }
    async fn run_migrations(&self) {
        match self {
            Self::Background(s) => s.run_migrations().await,
            Self::Http(s) => s.run_migrations().await,
        }
    }
}

pub struct HttpService {
    config: ServiceConfig,
    state: Option<AppState>,
    router_factory: fn(AppState) -> Router<AppState>,
}

impl HttpService {
    pub fn new(config_path: &str, router_factory: fn(AppState) -> Router<AppState>) -> Self {
        Self {
            config: ServiceConfig::from_file(config_path),
            state: None,
            router_factory,
        }
    }

    pub fn router(&self) -> Router {
        let state = self.state.clone().unwrap();
        get_router(
            &self.config,
            state.clone(),
            (self.router_factory)(state.clone()),
        )
    }
}

impl Service for HttpService {
    fn stub(self) -> Self {
        let config = ServiceConfig::stub();
        let state: AppState = AppState::new(&config);
        Self {
            config,
            state: Some(state),
            router_factory: self.router_factory,
        }
    }

    fn set_up(&mut self) {
        let state = AppState::new(&self.config);
        self.state = Some(state.clone());
    }

    async fn run(self) -> Result<(), std::io::Error> {
        let router = self.router();
        let addr = TcpListener::bind((self.config.ip, self.config.port))
            .await
            .unwrap();
        axum::serve(
            addr,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await
    }

    async fn run_migrations(&self) {
        let state = self.state.as_ref().unwrap();
        Migrator::new(std::path::Path::new("./migrations/principal"))
            .await
            .expect("Where are the migrations?")
            .run(&state.primary_database)
            .await
            .expect("Migrations failed");

        Migrator::new(std::path::Path::new("./migrations/events"))
            .await
            .expect("Where are the migrations?")
            .run(state.events_broker.storage())
            .await
            .expect("Migrations failed");
    }
}

pub trait Service {
    fn stub(self) -> Self;
    fn set_up(&mut self) {}
    fn run(self) -> impl std::future::Future<Output = Result<(), std::io::Error>>;
    fn run_migrations(&self) -> impl std::future::Future<Output = ()> {
        async {}
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
