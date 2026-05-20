//! HTTP client abstraction for wallet services.
//!
//! On native targets, uses `reqwest` for HTTP requests.
//! On wasm32, uses `web_sys::fetch` for browser-compatible HTTP requests.
//!
//! This module exists to decouple wallet services from the underlying HTTP
//! implementation, enabling the same service API to compile for both native
//! and WASM targets.

use serde::{de::DeserializeOwned, Serialize};

/// HTTP client abstraction.
#[derive(Debug, Clone)]
pub enum HttpClient {
    /// Native HTTP client backed by `reqwest`.
    #[cfg(not(target_arch = "wasm32"))]
    Native(crate::services::explorer::NativeClient),
    /// WASM HTTP client backed by `web_sys::fetch`.
    #[cfg(target_arch = "wasm32")]
    Wasm,
}

impl HttpClient {
    /// Create a new HTTP client with the given base URL.
    pub fn new(base_url: String) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::Native(crate::services::explorer::NativeClient::new(base_url))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = base_url;
            Self::Wasm
        }
    }

    /// Send a GET request and deserialize the response as JSON.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, HttpError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            match self {
                Self::Native(client) => client.get(path).await,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = path;
            Err(HttpError::WasmNotImplemented)
        }
    }

    /// Send a POST request with a JSON body and deserialize the response.
    pub async fn post<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, HttpError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            match self {
                Self::Native(client) => client.post(path, body).await,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (path, body);
            Err(HttpError::WasmNotImplemented)
        }
    }
}

/// HTTP error type.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("HTTP request failed: {0}")]
    Request(String),
    #[error("Failed to parse response: {0}")]
    Parse(String),
    #[error("WASM fetch not yet implemented — use native target or implement web_sys::fetch adapter")]
    WasmNotImplemented,
}

// ─────────────────────────────────────────────────────────────────────────────
// Native implementation (reqwest)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod native_impl {
    use super::*;
    use reqwest::Client as ReqwestClient;

    /// Native HTTP client backed by `reqwest`.
    #[derive(Debug)]
    pub struct NativeClient {
        base_url: String,
        client: ReqwestClient,
    }

    impl NativeClient {
        /// Create a new native HTTP client.
        pub fn new(base_url: String) -> Self {
            Self {
                base_url,
                client: ReqwestClient::new(),
            }
        }

        /// Send a GET request and deserialize the response as JSON.
        pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, HttpError> {
            let url = format!("{}{}", self.base_url, path);
            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| HttpError::Request(e.to_string()))?;

            if !response.status().is_success() {
                return Err(HttpError::Request(format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                )));
            }

            response
                .json::<T>()
                .await
                .map_err(|e| HttpError::Parse(e.to_string()))
        }

        /// Send a POST request with a JSON body and deserialize the response.
        pub async fn post<T: Serialize, R: DeserializeOwned>(
            &self,
            path: &str,
            body: &T,
        ) -> Result<R, HttpError> {
            let url = format!("{}{}", self.base_url, path);
            let response = self
                .client
                .post(&url)
                .json(body)
                .send()
                .await
                .map_err(|e| HttpError::Request(e.to_string()))?;

            if !response.status().is_success() {
                return Err(HttpError::Request(format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                )));
            }

            response
                .json::<R>()
                .await
                .map_err(|e| HttpError::Parse(e.to_string()))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_impl::NativeClient;

// ─────────────────────────────────────────────────────────────────────────────
// WASM implementation (web_sys::fetch)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Request;
    use web_sys::RequestInit;
    use web_sys::RequestMode;
    use web_sys::Response;

    /// WASM HTTP client backed by `web_sys::fetch`.
    #[derive(Debug)]
    pub struct WasmClient {
        base_url: String,
    }

    impl WasmClient {
        /// Create a new WASM HTTP client.
        pub fn new(base_url: String) -> Self {
            Self { base_url }
        }

        /// Send a GET request and deserialize the response as JSON.
        pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, HttpError> {
            let url = format!("{}{}", self.base_url, path);
            let opts = RequestInit::new();
            opts.set_method("GET");
            opts.set_mode(RequestMode::Cors);

            let window = web_sys::window().ok_or(HttpError::Request("No window".to_string()))?;
            let req = Request::new_with_str_and_init(&url, &opts).map_err(|e| HttpError::Request(format!("Request init failed: {:?}", e)))?;
            let resp_value = JsFuture::from(window.fetch_with_request(&req))
                .await
                .map_err(|e| HttpError::Request(format!("Fetch failed: {:?}", e)))?;

            let resp: Response = resp_value.dyn_into().map_err(|_| HttpError::Request("Invalid response".to_string()))?;
            let text = resp.text().map_err(|e| HttpError::Request(format!("Read body failed: {:?}", e)))?;
            let text = JsFuture::from(text).await.map_err(|e| HttpError::Request(format!("Text failed: {:?}", e)))?;
            let text: String = text.as_string().ok_or(HttpError::Request("Text not a string".to_string()))?;

            serde_json::from_str(&text).map_err(|e| HttpError::Parse(e.to_string()))
        }

        /// Send a POST request with a JSON body and deserialize the response.
        pub async fn post<T: Serialize, R: DeserializeOwned>(
            &self,
            path: &str,
            body: &T,
        ) -> Result<R, HttpError> {
            let url = format!("{}{}", self.base_url, path);
            let json = serde_json::to_string(body).map_err(|e| HttpError::Parse(e.to_string()))?;

            let opts = RequestInit::new();
            opts.set_method("POST");
            opts.set_mode(RequestMode::Cors);
            opts.set_body(&js_sys::Uint8Array::from(json.as_bytes()).into());

            let headers = web_sys::Headers::new().map_err(|_| HttpError::Request("Headers".to_string()))?;
            headers.set("Content-Type", "application/json").map_err(|_| HttpError::Request("Set header".to_string()))?;
            opts.set_headers(&headers);

            let window = web_sys::window().ok_or(HttpError::Request("No window".to_string()))?;
            let req = Request::new_with_str_and_init(&url, &opts).map_err(|e| HttpError::Request(format!("Request init failed: {:?}", e)))?;
            let resp_value = JsFuture::from(window.fetch_with_request(&req))
                .await
                .map_err(|e| HttpError::Request(format!("Fetch failed: {:?}", e)))?;

            let resp: Response = resp_value.dyn_into().map_err(|_| HttpError::Request("Invalid response".to_string()))?;
            let text = resp.text().map_err(|e| HttpError::Request(format!("Read body failed: {:?}", e)))?;
            let text = JsFuture::from(text).await.map_err(|e| HttpError::Request(format!("Text failed: {:?}", e)))?;
            let text: String = text.as_string().ok_or(HttpError::Request("Text not a string".to_string()))?;

            serde_json::from_str(&text).map_err(|e| HttpError::Parse(e.to_string()))
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_impl::WasmClient;
