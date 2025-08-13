use reqwest::Client;

#[derive(Debug, Clone)]
pub struct HttpClient {
    inner: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            inner: Client::new(),
        }
    }
}

impl std::ops::Deref for HttpClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
