use std::collections::HashMap;
use rmcp::{tool_handler, ErrorData, ServerHandler};
use rmcp::model::{CallToolResult, Content, ErrorCode, JsonObject, ServerCapabilities, ServerInfo, Tool};
use rmcp::handler::server::tool::{cached_schema_for_type, Parameters, ToolRoute, ToolRouter};
use reqwest::Url;
use futures_core::future::BoxFuture;
use rmcp::serde_json::Value;
use std::sync::Arc;
use clap::builder::Str;
use handlebars::{Context, Handlebars, JsonRender};
use rmcp::serde_json::value::RawValue;
use serde::de::IntoDeserializer;
use serde_valid::json::Map;
use tera::Tera;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema, Clone)]
pub struct DynamicMCPGeneralRequest {
    pub input: Value
}

#[derive(serde::Deserialize, Debug)]
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

#[derive(serde::Deserialize, Debug)]
struct HttpMetadata {
    url: String,
    method: HttpMethod,
    headers: Option<HashMap<String, String>>,
    input_schema: JsonObject,
    output_schema: Option<JsonObject>
}

#[derive(serde::Deserialize, Debug)]
pub struct ToolData {
    name: String,
    description: String,
    tool_type: ToolType,
    http_metadata: Option<HttpMetadata>
}

pub struct DynamicMCP {
    tool_router: ToolRouter<DynamicMCP>,
}

impl DynamicMCP {
    pub fn new_from_file(file_path: &str) -> Self {
        let data: Vec<ToolData> = match serde_yaml::from_str(file_path) {
            Ok(data) => data,
            Err(err) => panic!(
                "Error while parsing tool data file: {}",
                err
            )
        };
        Self::new(data)
    }

    pub fn new(tool_data: Vec<ToolData>) -> Self {
        Self {
            tool_router: Self::tool_router(tool_data),
        }
    }

    fn generate_tool_description(description: String, name: String, input_schema: JsonObject, output_schema: Option<JsonObject>) -> Tool {
        Tool {
            name: name.into(),
            description: Some(description.into()),
            input_schema: Arc::new(input_schema),
            output_schema: output_schema.map(|schema| Arc::new(schema)),
            annotations: None
        }
    }

    const URL_TEMPLATE_NAME: &'static str = "url";
    const INPUT_NAME: &'static str = "input";

    fn general_http_method_template<'a>(url: String, method: HttpMethod) -> impl Fn(Parameters<Value>) -> BoxFuture<'a, Result<CallToolResult, ErrorData>> {
        let mut handlebars = Handlebars::new();
        match handlebars.register_template_string(Self::URL_TEMPLATE_NAME, url) {
            Ok(_) => {}
            Err(err) => panic!("Error while registering template: {}", err)
        }
        move |Parameters(object): Parameters<Value>| -> BoxFuture<'a, Result<CallToolResult, ErrorData>> {
            let mut context_map = Map::new();
            context_map.insert(Self::INPUT_NAME.to_string(), object);
            let url = handlebars.render(Self::URL_TEMPLATE_NAME, &context_map).unwrap();
            let method = method.clone();
            Box::pin(async move {
                let client = reqwest::Client::new();
                let req = match method {
                    HttpMethod::GET => client.get(url),
                    HttpMethod::POST => client.post(url),
                    HttpMethod::PUT => client.put(url),
                    HttpMethod::DELETE => client.delete(url),
                };
                return  {
                        let res = req.send().await;
                        match res {
                            Ok(res_data) => {
                                // let test = res_data.json().await;
                                match res_data.text().await {
                                    Ok(res_text) => {
                                        Ok(CallToolResult::success(vec![Content::text(
                                            Value::String(res_text).to_string(),
                                        )]))
                                    }
                                    Err(err) => {
                                        Err(ErrorData::new(
                                            ErrorCode::INTERNAL_ERROR,
                                            format!("Error while reading response: {}", err),
                                            None))
                                    }
                                }
                            }
                            Err(err) => {
                                Err(ErrorData::new(
                                    ErrorCode::INTERNAL_ERROR,
                                    format!("Error while sending request: {}", err),
                                    None))
                            }
                        }
                }
            })
        }
    }

    pub fn tool_router(tool_data: Vec<ToolData>) -> ToolRouter<DynamicMCP> {
        let mut router = ToolRouter::new();

        for entry in tool_data {
            let function_tool = match entry.tool_type {
                ToolType::HTTP => {
                    let http_metadata = entry.http_metadata.unwrap();
                    Self::general_http_method_template(http_metadata.url, http_metadata.method)
                }
            };
            // router = router.with_route(ToolRoute::new(Self::generate_tool_description(description, entry.name), crate::core::closure::DynamicMCPClosure::new(function_tool)));
        }

        router
    }
}

#[tool_handler]
impl ServerHandler for DynamicMCP {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("This is a tool for getting weather information".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}