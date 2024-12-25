# Stefn
Fast and cool meta framework.

Simple and easy.

```rust
mod dashboard;
mod website;

use axum::{middleware::from_fn_with_state, Router};
use hyper::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method,
};

use stefn::{
    auth::{login_required_middleware, sessions_middleware},
    service::Service,
    state::WebsiteState,
};

use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

pub fn create_service() -> Service {
    Service::website("WEB_", routes)
}

fn routes(state: WebsiteState) -> Router<WebsiteState> {
    Router::new()
        .merge(dashboard::routes(state.clone()))
        .layer(from_fn_with_state(state.clone(), login_required_middleware))
        .merge(website::routes(state.clone()))
        .layer(from_fn_with_state(state.clone(), sessions_middleware))
        .nest_service("/dist", ServeDir::new("dist"))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::PUT])
                .allow_headers([CONTENT_TYPE, AUTHORIZATION])
                .allow_origin(Any),
        )
        .with_state(state)
}
```

```rust
use my_app::create_service;
use stefn::orquestrator::ServicesOrquestrator;

fn main() {
    ServicesOrquestrator::default()
        .load_environment_variables()
        .set_config_from_env()
        .enable_migrations()
        .add_service(create_service())
        .init_dev_tracing()
        .run();
}
```