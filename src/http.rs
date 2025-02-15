use std::{marker::PhantomData, ops::Deref};

pub struct Stub;

#[derive(Clone, Debug)]
pub struct HttpClient<S=()>{
    client: reqwest::Client,
    state: PhantomData<S>,
}

impl Deref for HttpClient {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self { client:reqwest::Client::new(), state: PhantomData }
    }

    pub fn stub() -> HttpClient<Stub> {
        HttpClient::<Stub> { client:reqwest::Client::new(), state: PhantomData }
    }
}

impl<Stub> HttpClient<Stub> {
    pub async fn get() -> () {

    }

}