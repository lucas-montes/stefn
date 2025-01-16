use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::connect_info::MockConnectInfo,
    http::{Request, Response},
};
use http_body_util::BodyExt;

use serde::Deserialize;

use crate::state::SharedState;

use tower::{Service, ServiceExt}; // for `call`, `oneshot`, and `ready`

pub struct StubService(super::Service);

impl StubService {
    pub async fn new(service: super::Service) -> Self {
        let mut service = super::ServiceExt::stub(service);
        let state = SharedState::stub();
        state.database().run_migrations().await;
        // state.events_broker().run_migrations().await;
        super::ServiceExt::set_up(&mut service, state).await;
        Self(service)
    }

    // pub async fn server(&self){
    //    let x = self.0
    //     .router()
    //     .unwrap()
    //     .clone()
    //     .layer(MockConnectInfo(SocketAddr::from(([0, 0, 0, 0], 8000))))
    //     .into_service()
    //     .ready()
    //     .await
    //     .unwrap()
    //     .to_owned();
    // }

    pub async fn request(&self, req: Request<Body>) -> Response<Body> {
        self.0
            .router()
            .unwrap()
            .clone()
            .layer(MockConnectInfo(SocketAddr::from(([0, 0, 0, 0], 8000))))
            .into_service()
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .unwrap()
        // self.0
        //     .router()
        //     .unwrap()
        //     .clone()
        //     .layer(MockConnectInfo(SocketAddr::from(([0, 0, 0, 0], 8000))))
        //     .oneshot(req)
        //     .await
        //     .unwrap()
    }

    pub async fn deserialize_response<T: for<'a> Deserialize<'a>>(response: Response<Body>) -> T {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }
}
