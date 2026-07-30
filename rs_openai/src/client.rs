use crate::apis::{
    assistants, audio, batch, chat, completions, edits, embeddings, engines, files, fine_tuning,
    images, messages, models, moderations, runs, threads, uploads,
};
use crate::shared::response_wrapper::{ApiErrorResponse, OpenAIError, OpenAIResponse};
use futures::{Stream, stream::StreamExt};
use reqwest::{
    Method,
    header::{HeaderMap, HeaderValue},
    multipart::Form,
};
use reqwest_eventsource::{Event, EventSource, RequestBuilderExt};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use serde::{Serialize, de::DeserializeOwned};
use std::fs::File;
use std::io;
use std::{fmt::Debug, pin::Pin};

/// Default v1 API base url
pub const API_BASE: &str = "https://api.openai.com/v1";

/// Name for organization header
pub const ORGANIZATION_HEADER: &str = "OpenAI-Organization";

/// Name for project header
pub const PROJECT_HEADER: &str = "OpenAI-Project";

pub struct OpenAI {
    api_key: String,
    org_id: Option<String>,
    project_id: Option<String>,
    base_url: String,
    /// Retried, connection-pooled client used for every non-streaming request.
    http: ClientWithMiddleware,
    /// Plain client used only for SSE streaming: `reqwest-eventsource` only
    /// extends `reqwest::RequestBuilder`, not the middleware-wrapped one.
    raw: reqwest::Client,
}

impl OpenAI {
    /// Transient failures (5xx, timeouts, connect errors) are retried automatically
    /// (up to 3 attempts, exponential backoff) on every request, including POST. This
    /// can duplicate side effects if a request is processed server-side but the
    /// response is lost in transit — matching the behavior of OpenAI's other official
    /// SDKs (`openai-python`, `openai-node`), which retry the same way by default.
    pub fn new(api_key: impl Into<String>) -> Self {
        let raw = reqwest::Client::new();
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let http = ClientBuilder::new(raw.clone())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        Self {
            api_key: api_key.into(),
            org_id: None,
            project_id: None,
            base_url: API_BASE.to_string(),
            http,
            raw,
        }
    }

    /// Sets the `OpenAI-Organization` header sent with every request.
    pub fn with_org_id(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Sets the `OpenAI-Project` header sent with every request.
    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Overrides the API base URL. Used by tests to point at a mock server;
    /// also usable for Azure OpenAI-compatible or proxy endpoints.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn headers(&self) -> Result<HeaderMap, OpenAIError> {
        let mut headers = HeaderMap::new();

        if let Some(org_id) = &self.org_id {
            headers.insert(
                ORGANIZATION_HEADER,
                HeaderValue::from_str(org_id)
                    .map_err(|e| OpenAIError::InvalidHeaderValue(e.to_string()))?,
            );
        }

        if let Some(project_id) = &self.project_id {
            headers.insert(
                PROJECT_HEADER,
                HeaderValue::from_str(project_id)
                    .map_err(|e| OpenAIError::InvalidHeaderValue(e.to_string()))?,
            );
        }

        Ok(headers)
    }

    /// Builds a request on the retried, connection-pooled client.
    fn openai_request(
        &self,
        method: Method,
        route: &str,
    ) -> Result<reqwest_middleware::RequestBuilder, OpenAIError> {
        Ok(self
            .http
            .request(method, format!("{}{route}", self.base_url))
            .headers(self.headers()?)
            .bearer_auth(&self.api_key))
    }

    /// Builds a request on the plain client. Used only by `post_stream`.
    fn raw_request(
        &self,
        method: Method,
        route: &str,
    ) -> Result<reqwest::RequestBuilder, OpenAIError> {
        Ok(self
            .raw
            .request(method, format!("{}{route}", self.base_url))
            .headers(self.headers()?)
            .bearer_auth(&self.api_key))
    }

    async fn resolve_response<T>(request: reqwest_middleware::RequestBuilder) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
    {
        let response = request.send().await?;
        let status = response.status();
        let bytes = response.bytes().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        let data: T =
            serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

        Ok(data)
    }

    /// Same as `resolve_response`, but for requests built on the plain (non-retried) client —
    /// see `post_form`'s doc comment for why multipart uploads need this instead.
    async fn resolve_raw_response<T>(request: reqwest::RequestBuilder) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
    {
        let response = request.send().await?;
        let status = response.status();
        let bytes = response.bytes().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        let data: T =
            serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

        Ok(data)
    }

    /// Same shape as `resolve_raw_response`, but returns the raw response body as text instead
    /// of deserializing it — see `post_form`'s doc comment for why multipart uploads need to
    /// build their request on the plain (non-retried) client.
    async fn resolve_raw_text_response(request: reqwest::RequestBuilder) -> OpenAIResponse<String> {
        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(text.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        Ok(text)
    }

    async fn resolve_file_response(
        request: reqwest_middleware::RequestBuilder,
        filename: &str,
    ) -> OpenAIResponse<()> {
        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(text.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        let mut file = File::create(filename)?;
        io::copy(&mut text.as_bytes(), &mut file)?;

        Ok(())
    }

    pub(crate) async fn get<T, F>(&self, route: &str, query: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.openai_request(Method::GET, route)?.query(query);
        Self::resolve_response(request).await
    }

    pub(crate) async fn post<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.openai_request(Method::POST, route)?.json(json);
        Self::resolve_response(request).await
    }

    /// Builds a request on the plain (non-retried) client. `reqwest::multipart::Form` always
    /// encodes its parts into a single interleaved byte stream (`Body::stream`, an
    /// `Inner::Streaming` body) regardless of whether individual `Part`s were built with
    /// `Part::bytes` or `Part::stream` — so multipart request bodies are never cloneable.
    /// `RetryTransientMiddleware` calls `req.try_clone()` unconditionally on every attempt,
    /// including the first, and errors immediately if that returns `None`. That means a
    /// multipart request sent through the retried `http` client fails 100% of the time, before
    /// any network I/O, against both a mock server and the real API — this is a hard limitation
    /// of `reqwest`/`reqwest-middleware`, not something configurable away. So, like
    /// `post_stream`, multipart uploads intentionally bypass the retried client.
    pub(crate) async fn post_form<T>(&self, route: &str, form_data: Form) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
    {
        let request = self.raw_request(Method::POST, route)?.multipart(form_data);
        Self::resolve_raw_response(request).await
    }

    /// See `post_form`'s doc comment: multipart bodies can't be retried, so this also uses the
    /// plain (non-retried) client.
    pub(crate) async fn post_form_with_text_response(
        &self,
        route: &str,
        form_data: Form,
    ) -> OpenAIResponse<String> {
        let request = self.raw_request(Method::POST, route)?.multipart(form_data);
        Self::resolve_raw_text_response(request).await
    }

    pub(crate) async fn post_with_file_response<T>(
        &self,
        route: &str,
        json: &T,
        filename: &str,
    ) -> OpenAIResponse<()>
    where
        T: Serialize,
    {
        let request = self.openai_request(Method::POST, route)?.json(json);
        Self::resolve_file_response(request, filename).await
    }

    pub(crate) async fn post_stream<T, F>(
        &self,
        route: &str,
        json: &F,
    ) -> OpenAIResponse<Pin<Box<dyn Stream<Item = OpenAIResponse<T>> + Send>>>
    where
        T: DeserializeOwned + Debug + Send + 'static,
        F: Serialize,
    {
        let request = self.raw_request(Method::POST, route)?.json(json);
        let event_source = request
            .eventsource()
            .map_err(|e| OpenAIError::StreamError(e.to_string()))?;
        Ok(OpenAI::stream_sse(event_source).await)
    }

    pub(crate) async fn delete<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.openai_request(Method::DELETE, route)?.json(json);
        Self::resolve_response(request).await
    }

    async fn stream_sse<T>(
        mut event_source: EventSource,
    ) -> Pin<Box<dyn Stream<Item = OpenAIResponse<T>> + Send>>
    where
        T: DeserializeOwned + Debug + Send + 'static,
    {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<OpenAIResponse<T>>();

        tokio::spawn(async move {
            while let Some(evt) = event_source.next().await {
                match evt {
                    Err(e) => {
                        if tx
                            .send(Err(OpenAIError::StreamError(e.to_string())))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(evt) => match evt {
                        Event::Message(message) => {
                            if message.data == "[DONE]" {
                                break;
                            }

                            let response = match serde_json::from_str::<T>(&message.data) {
                                Err(e) => Err(OpenAIError::JSONDeserialize(e)),
                                Ok(output) => Ok(output),
                            };

                            if tx.send(response).is_err() {
                                break;
                            }
                        }
                        Event::Open => continue,
                    },
                }
            }

            event_source.close();
        });

        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
    }

    pub fn assistants(&self) -> assistants::Assistants<'_> {
        assistants::Assistants::new(self)
    }

    pub fn audio(&self) -> audio::Audio<'_> {
        audio::Audio::new(self)
    }

    pub fn batch(&self) -> batch::Batch<'_> {
        batch::Batch::new(self)
    }

    pub fn chat(&self) -> chat::Chat<'_> {
        chat::Chat::new(self)
    }

    pub fn completions(&self) -> completions::Completions<'_> {
        completions::Completions::new(self)
    }

    pub fn edits(&self) -> edits::Edits<'_> {
        edits::Edits::new(self)
    }

    pub fn embeddings(&self) -> embeddings::Embeddings<'_> {
        embeddings::Embeddings::new(self)
    }

    pub fn engines(&self) -> engines::Engines<'_> {
        engines::Engines::new(self)
    }

    pub fn files(&self) -> files::Files<'_> {
        files::Files::new(self)
    }

    pub fn fine_tuning(&self) -> fine_tuning::FineTuning<'_> {
        fine_tuning::FineTuning::new(self)
    }

    pub fn images(&self) -> images::Images<'_> {
        images::Images::new(self)
    }

    pub fn messages(&self) -> messages::Messages<'_> {
        messages::Messages::new(self)
    }

    pub fn models(&self) -> models::Models<'_> {
        models::Models::new(self)
    }

    pub fn moderations(&self) -> moderations::Moderations<'_> {
        moderations::Moderations::new(self)
    }

    pub fn runs(&self) -> runs::Runs<'_> {
        runs::Runs::new(self)
    }

    pub fn threads(&self) -> threads::Threads<'_> {
        threads::Threads::new(self)
    }

    pub fn upload(&self) -> uploads::Upload<'_> {
        uploads::Upload::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn new_sets_api_key_and_no_org_or_project() {
        let client = OpenAI::new("sk-test");
        assert_eq!(client.api_key, "sk-test");
        assert!(client.org_id.is_none());
        assert!(client.project_id.is_none());
    }

    #[test]
    fn with_org_and_project_populate_headers() {
        let client = OpenAI::new("sk-test")
            .with_org_id("org-123")
            .with_project_id("proj-456");
        let headers = client.headers().unwrap();
        assert_eq!(headers.get(ORGANIZATION_HEADER).unwrap(), "org-123");
        assert_eq!(headers.get(PROJECT_HEADER).unwrap(), "proj-456");
    }

    #[tokio::test]
    async fn retries_transient_server_error_then_succeeds() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "object": "list", "data": [] })),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res: OpenAIResponse<serde_json::Value> = client.get("/models", &()).await;

        assert!(res.is_ok(), "expected the retry to succeed: {res:?}");
    }

    #[tokio::test]
    async fn maps_non_success_status_to_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "message": "Incorrect API key provided",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res: OpenAIResponse<serde_json::Value> = client.get("/models", &()).await;

        match res {
            Err(OpenAIError::ApiError(e)) => {
                assert_eq!(e.error.message, "Incorrect API key provided");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
