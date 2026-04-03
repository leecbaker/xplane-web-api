//! Typed websocket message models and a small async websocket client.
//!
//! This module mirrors the documented X-Plane websocket payload shapes for
//! API v3 and provides a minimal convenience client for send/receive flows.
#![allow(missing_docs)]

use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite};

/// Default websocket URL for the local X-Plane API v3 endpoint.
pub const DEFAULT_WS_API_URL: &str = "ws://localhost:8086/api/v3";

/// Errors produced while connecting to, sending to, or receiving from websocket
/// API.
#[derive(Debug, Error)]
pub enum WebSocketApiError {
    #[error("Failed to connect to websocket API: {source}")]
    Connect {
        #[source]
        source: tungstenite::Error,
    },

    #[error("Failed to send or receive websocket message: {source}")]
    Transport {
        #[source]
        source: tungstenite::Error,
    },

    #[error("Failed to serialize websocket payload: {source}")]
    Serialize {
        #[source]
        source: serde_json::Error,
    },

    #[error("Failed to parse UTF-8 websocket binary payload: {source}")]
    BinaryUtf8 {
        #[source]
        source: std::str::Utf8Error,
    },

    #[error("Failed to deserialize websocket payload `{payload}`: {source}")]
    Deserialize {
        payload: String,
        #[source]
        source: serde_json::Error,
    },
}

/// Result alias for websocket API operations.
pub type Result<T> = std::result::Result<T, WebSocketApiError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum All {
    All,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AllOrList<T> {
    All(All),
    List(Vec<T>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DatarefIndex {
    Single(u64),
    Multiple(Vec<u64>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatarefSelection {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<DatarefIndex>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatarefValueSet {
    pub id: u64,
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandSelection {
    pub id: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandSetActive {
    pub id: u64,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatarefSubscribeParams {
    pub datarefs: Vec<DatarefSelection>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatarefUnsubscribeParams {
    pub datarefs: AllOrList<DatarefSelection>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatarefSetValuesParams {
    pub datarefs: Vec<DatarefValueSet>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandSubscribeParams {
    pub commands: Vec<CommandSelection>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandUnsubscribeParams {
    pub commands: AllOrList<CommandSelection>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandSetActiveParams {
    pub commands: Vec<CommandSetActive>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "dataref_subscribe_values")]
    DatarefSubscribeValues {
        req_id: u64,
        params: DatarefSubscribeParams,
    },

    #[serde(rename = "dataref_unsubscribe_values")]
    DatarefUnsubscribeValues {
        req_id: u64,
        params: DatarefUnsubscribeParams,
    },

    #[serde(rename = "dataref_set_values")]
    DatarefSetValues {
        req_id: u64,
        params: DatarefSetValuesParams,
    },

    #[serde(rename = "command_subscribe_is_active")]
    CommandSubscribeIsActive {
        req_id: u64,
        params: CommandSubscribeParams,
    },

    #[serde(rename = "command_unsubscribe_is_active")]
    CommandUnsubscribeIsActive {
        req_id: u64,
        params: CommandUnsubscribeParams,
    },

    #[serde(rename = "command_set_is_active")]
    CommandSetIsActive {
        req_id: u64,
        params: CommandSetActiveParams,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "result")]
    Result {
        req_id: u64,
        success: bool,
        #[serde(default)]
        error_code: Option<String>,
        #[serde(default)]
        error_message: Option<String>,
    },

    #[serde(rename = "dataref_update_values")]
    DatarefUpdateValues { data: HashMap<String, Value> },

    #[serde(rename = "command_update_is_active")]
    CommandUpdateIsActive { data: HashMap<String, bool> },
}

/// Convenience client for typed websocket message exchange with X-Plane.
pub struct WebSocketApiClient {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_req_id: u64,
}

impl WebSocketApiClient {
    /// Opens a websocket connection to the provided URL.
    pub async fn connect(url: &str) -> Result<Self> {
        let (stream, _response) = connect_async(url)
            .await
            .map_err(|source| WebSocketApiError::Connect { source })?;
        Ok(Self {
            stream,
            next_req_id: 1,
        })
    }

    /// Overrides the next request ID used by helper send methods.
    pub fn set_next_req_id(&mut self, next_req_id: u64) {
        self.next_req_id = next_req_id;
    }

    /// Sends a typed client message as JSON text.
    pub async fn send_message(&mut self, message: &ClientMessage) -> Result<()> {
        let payload = serde_json::to_string(message)
            .map_err(|source| WebSocketApiError::Serialize { source })?;
        self.stream
            .send(Message::Text(payload.into()))
            .await
            .map_err(|source| WebSocketApiError::Transport { source })
    }

    /// Receives and parses the next server message.
    ///
    /// Returns `Ok(None)` when the remote closes the connection or the stream
    /// ends.
    pub async fn recv_message(&mut self) -> Result<Option<ServerMessage>> {
        loop {
            let Some(message_result) = self.stream.next().await else {
                return Ok(None);
            };

            let message =
                message_result.map_err(|source| WebSocketApiError::Transport { source })?;
            match message {
                Message::Text(payload) => {
                    let payload = payload.to_string();
                    let parsed =
                        serde_json::from_str::<ServerMessage>(&payload).map_err(|source| {
                            WebSocketApiError::Deserialize {
                                payload: payload.clone(),
                                source,
                            }
                        })?;
                    return Ok(Some(parsed));
                }
                Message::Binary(payload) => {
                    let payload_str = std::str::from_utf8(&payload)
                        .map_err(|source| WebSocketApiError::BinaryUtf8 { source })?
                        .to_string();
                    let parsed =
                        serde_json::from_str::<ServerMessage>(&payload_str).map_err(|source| {
                            WebSocketApiError::Deserialize {
                                payload: payload_str.clone(),
                                source,
                            }
                        })?;
                    return Ok(Some(parsed));
                }
                Message::Ping(_) | Message::Pong(_) => {
                    continue;
                }
                Message::Close(_) => {
                    return Ok(None);
                }
                _ => {
                    continue;
                }
            }
        }
    }

    /// Sends a normal websocket close frame and consumes the client.
    pub async fn close(mut self) -> Result<()> {
        self.stream
            .close(None)
            .await
            .map_err(|source| WebSocketApiError::Transport { source })
    }

    /// Sends a `dataref_subscribe_values` message and returns the request ID.
    pub async fn dataref_subscribe_values(
        &mut self,
        datarefs: Vec<DatarefSelection>,
    ) -> Result<u64> {
        let req_id = self.reserve_req_id();
        self.send_message(&ClientMessage::DatarefSubscribeValues {
            req_id,
            params: DatarefSubscribeParams { datarefs },
        })
        .await?;
        Ok(req_id)
    }

    /// Sends a `dataref_unsubscribe_values` message and returns the request ID.
    pub async fn dataref_unsubscribe_values(
        &mut self,
        datarefs: AllOrList<DatarefSelection>,
    ) -> Result<u64> {
        let req_id = self.reserve_req_id();
        self.send_message(&ClientMessage::DatarefUnsubscribeValues {
            req_id,
            params: DatarefUnsubscribeParams { datarefs },
        })
        .await?;
        Ok(req_id)
    }

    /// Sends a `dataref_set_values` message and returns the request ID.
    pub async fn dataref_set_values(&mut self, datarefs: Vec<DatarefValueSet>) -> Result<u64> {
        let req_id = self.reserve_req_id();
        self.send_message(&ClientMessage::DatarefSetValues {
            req_id,
            params: DatarefSetValuesParams { datarefs },
        })
        .await?;
        Ok(req_id)
    }

    /// Sends a `command_subscribe_is_active` message and returns the request
    /// ID.
    pub async fn command_subscribe_is_active(
        &mut self,
        commands: Vec<CommandSelection>,
    ) -> Result<u64> {
        let req_id = self.reserve_req_id();
        self.send_message(&ClientMessage::CommandSubscribeIsActive {
            req_id,
            params: CommandSubscribeParams { commands },
        })
        .await?;
        Ok(req_id)
    }

    /// Sends a `command_unsubscribe_is_active` message and returns the request
    /// ID.
    pub async fn command_unsubscribe_is_active(
        &mut self,
        commands: AllOrList<CommandSelection>,
    ) -> Result<u64> {
        let req_id = self.reserve_req_id();
        self.send_message(&ClientMessage::CommandUnsubscribeIsActive {
            req_id,
            params: CommandUnsubscribeParams { commands },
        })
        .await?;
        Ok(req_id)
    }

    /// Sends a `command_set_is_active` message and returns the request ID.
    pub async fn command_set_is_active(&mut self, commands: Vec<CommandSetActive>) -> Result<u64> {
        let req_id = self.reserve_req_id();
        self.send_message(&ClientMessage::CommandSetIsActive {
            req_id,
            params: CommandSetActiveParams { commands },
        })
        .await?;
        Ok(req_id)
    }

    fn reserve_req_id(&mut self) -> u64 {
        let req_id = self.next_req_id;
        self.next_req_id = self.next_req_id.saturating_add(1);
        req_id
    }
}
