use axum::Router;
use futures::future::BoxFuture;

use std::net::SocketAddr;
use tokio::{net::TcpListener, signal};

use crate::{
    config::{APIConfig, ServiceConfig, WebsiteConfig},
    state::SharedState,
    APIState, WebsiteState,
};

use super::get_router;

pub struct WebsiteService {
    config: WebsiteConfig,
    router_factory: fn(WebsiteState) -> Router<WebsiteState>,
    router: Option<Router>,
}

impl WebsiteService {
    fn new(env_prefix: &str, router_factory: fn(WebsiteState) -> Router<WebsiteState>) -> Self {
        Self {
            config: WebsiteConfig::from_env_with_prefix(env_prefix),
            router_factory,
            router: None,
        }
    }
}

impl ServiceExt for WebsiteService {
    fn stub(self) -> Self {
        Self {
            config: WebsiteConfig::stub(),
            router_factory: self.router_factory,
            router: None,
        }
    }

    fn set_up(&mut self, shared: SharedState) {
        let state = WebsiteState::new(self.config.clone(), shared);
        let routes = (self.router_factory)(state.clone());
        let router = get_router(state, routes);
        self.router = Some(router);
    }

    async fn run(self) -> Result<(), std::io::Error> {
        let addr = TcpListener::bind(self.config.socket_addr()).await.unwrap();
        axum::serve(
            addr,
            self.router
                .unwrap()
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await
    }
}

pub struct APIService {
    config: APIConfig,
    router_factory: fn(APIState) -> Router<APIState>,
    router: Option<Router>,
}

impl APIService {
    fn new(env_prefix: &str, router_factory: fn(APIState) -> Router<APIState>) -> Self {
        Self {
            config: APIConfig::from_env_with_prefix(env_prefix),
            router_factory,
            router: None,
        }
    }
}

impl ServiceExt for APIService {
    fn stub(self) -> Self {
        Self {
            config: APIConfig::stub(),
            router_factory: self.router_factory,
            router: None,
        }
    }

    fn set_up(&mut self, shared: SharedState) {
        let state = APIState::new(self.config.clone(), shared);
        let routes = (self.router_factory)(state.clone());
        let router = get_router(state, routes);
        self.router = Some(router);
    }

    async fn run(self) -> Result<(), std::io::Error> {
        let addr = TcpListener::bind(self.config.socket_addr()).await.unwrap();
        axum::serve(
            addr,
            self.router
                .unwrap()
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await
    }
}

pub enum Service {
    Background(BackgroundService),
    API(APIService),
    Website(WebsiteService),
}

impl Service {
    pub fn api(env_prefix: &str, router_factory: fn(APIState) -> Router<APIState>) -> Self {
        Self::API(APIService::new(env_prefix, router_factory))
    }
    pub fn website(
        env_prefix: &str,
        router_factory: fn(WebsiteState) -> Router<WebsiteState>,
    ) -> Self {
        Self::Website(WebsiteService::new(env_prefix, router_factory))
    }
    pub fn background(
        env_prefix: &str,
        task: fn() -> BoxFuture<'static, Result<(), std::io::Error>>,
    ) -> Self {
        Self::Background(BackgroundService::new(env_prefix, task))
    }
    pub fn router(&self) -> Option<&Router> {
        match self {
            Self::Background(_) => None,
            Self::API(s) => s.router.as_ref(),
            Self::Website(s) => s.router.as_ref(),
        }
    }
}

impl ServiceExt for Service {
    fn stub(self) -> Self {
        match self {
            Self::Background(s) => Self::Background(s.stub()),
            Self::API(s) => Self::API(s.stub()),
            Self::Website(s) => Self::Website(s.stub()),
        }
    }
    fn set_up(&mut self, shared: SharedState) {
        match self {
            Self::Background(s) => s.set_up(shared),
            Self::API(s) => s.set_up(shared),
            Self::Website(s) => s.set_up(shared),
        }
    }
    async fn run(self) -> Result<(), std::io::Error> {
        match self {
            Self::Background(s) => s.run().await,
            Self::API(s) => {
                s.config.print();
                s.run().await
            }
            Self::Website(s) => {
                s.config.print();
                s.run().await
            }
        }
    }
}

pub struct BackgroundService {
    task: Box<dyn Fn() -> BoxFuture<'static, Result<(), std::io::Error>> + Send>,
}

impl BackgroundService {
    fn new(env_prefix: &str, task: fn() -> BoxFuture<'static, Result<(), std::io::Error>>) -> Self {
        Self {
            task: Box::new(task),
        }
    }
}

impl ServiceExt for BackgroundService {
    fn stub(self) -> Self {
        Self { task: self.task }
    }

    async fn run(self) -> Result<(), std::io::Error> {
        (self.task)().await
    }
}

pub trait ServiceExt {
    fn stub(self) -> Self;
    fn set_up(&mut self, shared: SharedState) {}
    fn run(self) -> impl std::future::Future<Output = Result<(), std::io::Error>>;
}

pub async fn shutdown_signal() {
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
