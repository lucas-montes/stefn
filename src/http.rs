use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;

use serde::de::DeserializeOwned;

use http::StatusCode;
use reqwest::Url;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Body, Client, IntoUrl, Method, Request, Version,
};
use serde::Serialize;
use serde_json::Value;
use std::fmt;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Default, Debug)]
pub struct MockStore {
    responses: HashMap<String, Vec<Value>>,
}

pub struct RequestBuilder {
    builder: reqwest::RequestBuilder,
}

impl From<reqwest::RequestBuilder> for RequestBuilder {
    fn from(builder: reqwest::RequestBuilder) -> Self {
        RequestBuilder { builder }
    }
}

impl RequestBuilder {
    pub fn new(builder: reqwest::RequestBuilder) -> Self {
        Self { builder }
    }

    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self {
            builder: self.builder.header(key, value),
        }
    }

    pub fn headers(self, headers: HeaderMap) -> Self {
        Self {
            builder: self.builder.headers(headers),
        }
    }

    pub fn basic_auth<U, P>(self, username: U, password: Option<P>) -> Self
    where
        U: fmt::Display,
        P: fmt::Display,
    {
        Self {
            builder: self.builder.basic_auth(username, password),
        }
    }

    pub fn bearer_auth<T>(self, token: T) -> Self
    where
        T: fmt::Display,
    {
        Self {
            builder: self.builder.bearer_auth(token),
        }
    }

    pub fn body<T: Into<Body>>(self, body: T) -> Self {
        Self {
            builder: self.builder.body(body),
        }
    }

    pub fn timeout(self, timeout: Duration) -> Self {
        Self {
            builder: self.builder.timeout(timeout),
        }
    }

    #[cfg(feature = "multipart")]
    pub fn multipart(self, multipart: reqwest::multipart::Form) -> Self {
        Self {
            builder: self.builder.multipart(multipart),
        }
    }

    pub fn query<T: Serialize + ?Sized>(self, query: &T) -> Self {
        Self {
            builder: self.builder.query(query),
        }
    }

    pub fn version(self, version: Version) -> Self {
        Self {
            builder: self.builder.version(version),
        }
    }

    pub fn form<T: Serialize + ?Sized>(self, form: &T) -> Self {
        Self {
            builder: self.builder.form(form),
        }
    }

    pub fn json<T: Serialize + ?Sized>(self, json: &T) -> Self {
        Self {
            builder: self.builder.json(json),
        }
    }

    pub fn fetch_mode_no_cors(self) -> Self {
        Self {
            builder: self.builder.fetch_mode_no_cors(),
        }
    }

    pub fn build(self) -> reqwest::Result<Request> {
        self.builder.build()
    }

    pub fn build_split(self) -> (Client, reqwest::Result<Request>) {
        self.builder.build_split()
    }

    pub async fn send(self) -> reqwest::Result<Response> {
        self.builder.send().await.map(|r| r.into())
    }

    pub fn try_clone(&self) -> Option<Self> {
        self.builder.try_clone().map(|builder| Self { builder })
    }
}

#[derive(Debug, Clone)]
pub struct HttpClient {
    inner: Client,
    mock_store: Option<Arc<Mutex<MockStore>>>,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            inner: Client::new(),
            mock_store: None,
        }
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.inner.get(url).into()
    }

    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.inner.post(url).into()
    }

    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.inner.put(url).into()
    }

    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.inner.patch(url).into()
    }

    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.inner.delete(url).into()
    }

    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.inner.head(url).into()
    }

    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        self.inner.request(method, url).into()
    }

    pub async fn execute(&self, request: Request) -> Result<Response, reqwest::Error> {
        self.inner.execute(request).await.map(|r| r.into())
    }
}

impl std::ops::Deref for HttpClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<reqwest::Response> for Response {
    fn from(response: reqwest::Response) -> Self {
        Response { response }
    }
}

/// A Response to a submitted `Request`.
pub struct Response {
    response: reqwest::Response,
}

impl Response {
    pub fn new(response: reqwest::Response) -> Self {
        Response { response }
    }

    /// Get the `StatusCode` of this `Response`.
    #[inline]
    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    /// Get the HTTP `Version` of this `Response`.
    #[inline]
    pub fn version(&self) -> Version {
        self.response.version()
    }

    /// Get the `Headers` of this `Response`.
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.response.headers()
    }

    /// Get a mutable reference to the `Headers` of this `Response`.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.response.headers_mut()
    }

    /// Get the content-length of this response, if known.
    ///
    /// Reasons it may not be known:
    ///
    /// - The server didn't send a `content-length` header.
    /// - The response is compressed and automatically decoded (thus changing
    ///   the actual decoded length).
    pub fn content_length(&self) -> Option<u64> {
        self.response.content_length()
    }

    /// Retrieve the cookies contained in the response.
    ///
    /// Note that invalid 'Set-Cookie' headers will be ignored.
    ///
    /// # Optional
    ///
    /// This requires the optional `cookies` feature to be enabled.
    #[cfg(feature = "cookies")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cookies")))]
    pub fn cookies<'a>(&'a self) -> impl Iterator<Item = cookie::Cookie<'a>> + 'a {
        self.response.cookies()
    }

    /// Get the final `Url` of this `Response`.
    #[inline]
    pub fn url(&self) -> &Url {
        &self.response.url()
    }

    /// Get the remote address used to get this `Response`.
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.response.remote_addr()
    }

    /// Returns a reference to the associated extensions.
    pub fn extensions(&self) -> &http::Extensions {
        self.response.extensions()
    }

    /// Returns a mutable reference to the associated extensions.
    pub fn extensions_mut(&mut self) -> &mut http::Extensions {
        self.response.extensions_mut()
    }

    // body methods

    /// Get the full response text.
    ///
    /// This method decodes the response body with BOM sniffing
    /// and with malformed sequences replaced with the REPLACEMENT CHARACTER.
    /// Encoding is determined from the `charset` parameter of `Content-Type` header,
    /// and defaults to `utf-8` if not presented.
    ///
    /// Note that the BOM is stripped from the returned String.
    ///
    /// # Note
    ///
    /// If the `charset` feature is disabled the method will only attempt to decode the
    /// response as UTF-8, regardless of the given `Content-Type`
    ///
    /// # Example
    ///
    /// ```
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let content = reqwest::get("http://httpbin.org/range/26")
    ///     .await?
    ///     .text()
    ///     .await?;
    ///
    /// println!("text: {content:?}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn text(self) -> reqwest::Result<String> {
        self.response.text().await
    }

    /// Get the full response text given a specific encoding.
    ///
    /// This method decodes the response body with BOM sniffing
    /// and with malformed sequences replaced with the REPLACEMENT CHARACTER.
    /// You can provide a default encoding for decoding the raw message, while the
    /// `charset` parameter of `Content-Type` header is still prioritized. For more information
    /// about the possible encoding name, please go to [`encoding_rs`] docs.
    ///
    /// Note that the BOM is stripped from the returned String.
    ///
    /// [`encoding_rs`]: https://docs.rs/encoding_rs/0.8/encoding_rs/#relationship-with-windows-code-pages
    ///
    /// # Optional
    ///
    /// This requires the optional `encoding_rs` feature enabled.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let content = reqwest::get("http://httpbin.org/range/26")
    ///     .await?
    ///     .text_with_charset("utf-8")
    ///     .await?;
    ///
    /// println!("text: {content:?}");
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "charset")]
    #[cfg_attr(docsrs, doc(cfg(feature = "charset")))]
    pub async fn text_with_charset(self, default_encoding: &str) -> reqwest::Result<String> {
        self.response.text_with_charset().await
    }

    /// Try to deserialize the response body as JSON.
    ///
    /// # Optional
    ///
    /// This requires the optional `json` feature enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate reqwest;
    /// # extern crate serde;
    /// #
    /// # use reqwest::Error;
    /// # use serde::Deserialize;
    /// #
    /// // This `derive` requires the `serde` dependency.
    /// #[derive(Deserialize)]
    /// struct Ip {
    ///     origin: String,
    /// }
    ///
    /// # async fn run() -> Result<(), Error> {
    /// let ip = reqwest::get("http://httpbin.org/ip")
    ///     .await?
    ///     .json::<Ip>()
    ///     .await?;
    ///
    /// println!("ip: {}", ip.origin);
    /// # Ok(())
    /// # }
    /// #
    /// # fn main() { }
    /// ```
    ///
    /// # Errors
    ///
    /// This method fails whenever the response body is not in JSON format
    /// or it cannot be properly deserialized to target type `T`. For more
    /// details please see [`serde_json::from_reader`].
    ///
    /// [`serde_json::from_reader`]: https://docs.serde.rs/serde_json/fn.from_reader.html
    pub async fn json<T: DeserializeOwned>(self) -> reqwest::Result<T> {
        self.response.json().await
    }

    /// Get the full response body as `Bytes`.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let bytes = reqwest::get("http://httpbin.org/ip")
    ///     .await?
    ///     .bytes()
    ///     .await?;
    ///
    /// println!("bytes: {bytes:?}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bytes(self) -> reqwest::Result<Bytes> {
        self.response.bytes().await
    }

    /// Stream a chunk of the response body.
    ///
    /// When the response body has been exhausted, this will return `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut res = reqwest::get("https://hyper.rs").await?;
    ///
    /// while let Some(chunk) = res.chunk().await? {
    ///     println!("Chunk: {chunk:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chunk(&mut self) -> reqwest::Result<Option<Bytes>> {
        self.response.chunk().await
    }

    /// Convert the response into a `Stream` of `Bytes` from the body.
    ///
    /// # Example
    ///
    /// ```
    /// use futures_util::StreamExt;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stream = reqwest::get("http://httpbin.org/ip")
    ///     .await?
    ///     .bytes_stream();
    ///
    /// while let Some(item) = stream.next().await {
    ///     println!("Chunk: {:?}", item?);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Optional
    ///
    /// This requires the optional `stream` feature to be enabled.
    #[cfg(feature = "stream")]
    #[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
    pub fn bytes_stream(self) -> impl futures_core::Stream<Item = reqwest::Result<Bytes>> {
        self.response.bytes().await
    }

    // util methods

    /// Turn a response into an error if the server returned an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use reqwest::Response;
    /// fn on_response(res: Response) {
    ///     match res.error_for_status() {
    ///         Ok(_res) => (),
    ///         Err(err) => {
    ///             // asserting a 400 as an example
    ///             // it could be any status between 400...599
    ///             assert_eq!(
    ///                 err.status(),
    ///                 Some(reqwest::StatusCode::BAD_REQUEST)
    ///             );
    ///         }
    ///     }
    /// }
    /// # fn main() {}
    /// ```
    pub fn error_for_status(self) -> reqwest::Result<Self> {
        self.response.error_for_status().map(|r| r.into())
    }

    /// Turn a reference to a response into an error if the server returned an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use reqwest::Response;
    /// fn on_response(res: &Response) {
    ///     match res.error_for_status_ref() {
    ///         Ok(_res) => (),
    ///         Err(err) => {
    ///             // asserting a 400 as an example
    ///             // it could be any status between 400...599
    ///             assert_eq!(
    ///                 err.status(),
    ///                 Some(reqwest::StatusCode::BAD_REQUEST)
    ///             );
    ///         }
    ///     }
    /// }
    /// # fn main() {}
    /// ```
    pub fn error_for_status_ref(&self) -> reqwest::Result<&Self> {
        self.response.error_for_status_ref().map(|_| self)
    }
}
