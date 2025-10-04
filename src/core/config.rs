use rmcp::model::{Implementation, JsonObject, ServerCapabilities, ToolAnnotations};
use std::collections::HashMap;

#[derive(serde::Deserialize, Debug, Clone)]
pub enum ToolType {
    HTTP,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub enum TransportType {
    STDIO,
    SSE,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct HttpMetadata {
    pub url: String,
    pub method: HttpMethod,
    pub body_template: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub input_schema: JsonObject,
    pub output_schema: Option<JsonObject>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ToolData {
    pub name: String,
    pub description: String,
    pub tool_type: ToolType,
    pub http_metadata: Option<HttpMetadata>,
    pub tool_annotations: Option<ToolAnnotations>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct TransportConfig {
    pub transport_type: TransportType,
}

impl Default for TransportConfig {
    fn default() -> Self {
        TransportConfig {
            transport_type: TransportType::STDIO,
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct DynamicMCPConfig {
    pub tools: Vec<ToolData>,
    pub instruction: Option<String>,
    pub server_info: Option<Implementation>,
    pub server_capabilities: Option<ServerCapabilities>,
    pub transport_config: Option<TransportConfig>,
}

impl DynamicMCPConfig {
    pub async fn new_from_file(file_path: String) -> Self {
        let file_bytes = tokio::fs::read(file_path)
            .await
            .unwrap_or_else(|err| panic!("Error while reading the config file: {}", err));
        match serde_yaml::from_str(String::from_utf8(file_bytes).unwrap().as_str()) {
            Ok(data) => data,
            Err(err) => panic!("Error while parsing the config file: {}", err),
        }
    }
}
