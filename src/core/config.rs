use std::collections::HashMap;
use rmcp::model::JsonObject;

#[derive(serde::Deserialize, Debug, Clone)]
pub enum ToolType {
    HTTP
}

#[derive(serde::Deserialize, Debug, Clone)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct HttpMetadata {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Option<HashMap<String, String>>,
    pub input_schema: JsonObject,
    pub output_schema: Option<JsonObject>
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ToolData {
    pub name: String,
    pub description: String,
    pub tool_type: ToolType,
    pub http_metadata: Option<HttpMetadata>
}