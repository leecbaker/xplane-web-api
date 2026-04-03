//! Typed client crate for the documented X-Plane local web API.
//!
//! - REST API: generated from OpenAPI at build time with Progenitor.
//! - WebSocket API: typed request/response models and a small async client.
//! - CLI: optional command-line interface for REST operations.
//! - Error: shared typed REST error classification helpers.
//!
//! # Basic REST Example
//!
//! ```rust
//! use xplane_web_api::error::RestClientError;
//! use xplane_web_api::rest::{Client, DEFAULT_REST_API_BASE_URL};
//!
//! async fn fetch_capabilities() -> Result<(), RestClientError> {
//!     let client = Client::new(DEFAULT_REST_API_BASE_URL);
//!     let response = client
//!         .get_capabilities()
//!         .await
//!         .map_err(RestClientError::from)?;
//!
//!     println!("{:#?}", response.as_ref());
//!     Ok(())
//! }
//! ```

#![warn(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

/// Generated REST client and OpenAPI-derived request/response types.
#[allow(clippy::unwrap_used, missing_docs)]
pub mod rest {
    /// Default base URL for the local X-Plane REST API.
    pub const DEFAULT_REST_API_BASE_URL: &str = "http://localhost:8086";

    include!(concat!(env!("OUT_DIR"), "/xplane_web_api.rs"));
}

/// Shared REST error classification helpers for generated client operations.
pub mod error;

#[cfg(feature = "websocket")]
/// Typed websocket request/response models and an async convenience client.
pub mod websocket;
