use axum::Router;
use std::net::SocketAddr;
use tokio::{net::TcpListener, signal};

use super::{get_router, AppState, ServiceConfig};

#[derive(Clone)]
pub struct Service {
    config: ServiceConfig,
    router_factory: fn(AppState) -> Router<AppState>,
}

impl Service {
    pub fn new(config_path: &str, router_factory: fn(AppState) -> Router<AppState>) -> Self {
        Self {
            config: ServiceConfig::from_file(config_path),
            router_factory,
        }
    }

    pub fn stub(self) -> Self {
        Self {
            config: ServiceConfig::stub(),
            router_factory: self.router_factory,
        }
    }

    pub fn router(&self) -> Router {
        let state = AppState::new(&self.config);
        get_router(
            &self.config,
            state.clone(),
            (self.router_factory)(state.clone()),
        )
    }

    pub async fn run(&self) -> Result<(), std::io::Error> {
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
