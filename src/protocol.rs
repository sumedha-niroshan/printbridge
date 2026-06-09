use serde::{Deserialize, Serialize};

/// Incoming message from the browser SDK
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum IncomingMessage {
    ListPrinters { id: String },
    Print { id: String, payload: PrintPayload },
    Status { id: String },
    Ping { id: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintPayload {
    /// Name of the printer (from listPrinters)
    pub printer: String,
    /// "raw" | "escpos" | "text"
    #[serde(rename = "type")]
    pub print_type: PrintType,
    /// Base64-encoded data for raw/escpos, plain string for text
    pub data: String,
    pub copies: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PrintType {
    Raw,
    Escpos,
    Text,
}

/// Outgoing message to the browser SDK
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingMessage {
    pub id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
    pub is_online: bool,
}

impl OutgoingMessage {
    pub fn ok(id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}
