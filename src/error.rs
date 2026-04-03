//! REST client error classification helpers.

use thiserror::Error;

use crate::rest;

/// Unified error type for generated REST client calls.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum RestClientError {
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("reqwest timeout: {source}")]
    ReqwestTimeout {
        #[source]
        source: reqwest::Error,
    },

    #[error("reqwest connect error: {source}")]
    ReqwestConnect {
        #[source]
        source: reqwest::Error,
    },

    #[error("reqwest request construction error: {source}")]
    ReqwestRequest {
        #[source]
        source: reqwest::Error,
    },

    #[error("reqwest redirect error: {source}")]
    ReqwestRedirect {
        #[source]
        source: reqwest::Error,
    },

    #[error("reqwest HTTP status error: {source}")]
    ReqwestStatus {
        #[source]
        source: reqwest::Error,
    },

    #[error("reqwest response body error: {source}")]
    ReqwestBody {
        #[source]
        source: reqwest::Error,
    },

    #[error("reqwest decode error: {source}")]
    ReqwestDecode {
        #[source]
        source: reqwest::Error,
    },

    #[error("reqwest error: {source}")]
    ReqwestOther {
        #[source]
        source: reqwest::Error,
    },

    #[error("documented API error response: HTTP {status} {error_code}: {error_message}")]
    ApiErrorResponse {
        status: reqwest::StatusCode,
        error_code: String,
        error_message: String,
    },

    #[error("undocumented error response: HTTP {status}")]
    UndocumentedErrorResponse { status: reqwest::StatusCode },

    #[error("invalid response payload: {source}")]
    InvalidResponsePayload {
        #[source]
        source: serde_json::Error,
    },

    #[error("unexpected response: HTTP {status}")]
    UnexpectedResponse { status: reqwest::StatusCode },

    #[error("{message}")]
    Custom { message: String },
}

fn reqwest_error(source: reqwest::Error) -> RestClientError {
    if source.is_timeout() {
        RestClientError::ReqwestTimeout { source }
    } else if source.is_connect() {
        RestClientError::ReqwestConnect { source }
    } else if source.is_request() {
        RestClientError::ReqwestRequest { source }
    } else if source.is_redirect() {
        RestClientError::ReqwestRedirect { source }
    } else if source.is_status() {
        RestClientError::ReqwestStatus { source }
    } else if source.is_body() {
        RestClientError::ReqwestBody { source }
    } else if source.is_decode() {
        RestClientError::ReqwestDecode { source }
    } else {
        RestClientError::ReqwestOther { source }
    }
}

fn map_rest_error<T>(
    source: rest::Error<T>,
    to_api_fields: impl FnOnce(T) -> Option<(String, String)>,
) -> RestClientError {
    match source {
        rest::Error::InvalidRequest(message) => RestClientError::InvalidRequest { message },
        rest::Error::CommunicationError(source)
        | rest::Error::InvalidUpgrade(source)
        | rest::Error::ResponseBodyError(source) => reqwest_error(source),
        rest::Error::ErrorResponse(response) => {
            let status = response.status();
            let response_body = response.into_inner();
            if let Some((error_code, error_message)) = to_api_fields(response_body) {
                RestClientError::ApiErrorResponse {
                    status,
                    error_code,
                    error_message,
                }
            } else {
                RestClientError::UndocumentedErrorResponse { status }
            }
        }
        rest::Error::InvalidResponsePayload(_, source) => {
            RestClientError::InvalidResponsePayload { source }
        }
        rest::Error::UnexpectedResponse(response) => RestClientError::UnexpectedResponse {
            status: response.status(),
        },
        rest::Error::Custom(message) => RestClientError::Custom { message },
    }
}

impl From<rest::Error<()>> for RestClientError {
    fn from(source: rest::Error<()>) -> Self {
        map_rest_error(source, |_| None)
    }
}

impl From<rest::Error<rest::types::ApiError>> for RestClientError {
    fn from(source: rest::Error<rest::types::ApiError>) -> Self {
        map_rest_error(source, |api_error| {
            Some((api_error.error_code, api_error.error_message))
        })
    }
}
