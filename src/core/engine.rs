use crate::core::closure::DynamicMCPClosure;
use crate::core::config::{DynamicMCPConfig, HttpMethod, ToolData, ToolType};
use crate::core::template::Template;
use futures_core::future::BoxFuture;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use reqwest::Body;
use rmcp::handler::server::tool::{Parameters, ToolRoute, ToolRouter};
use rmcp::model::{
    CallToolResult, Content, ErrorCode, Implementation, JsonObject, ServerCapabilities, ServerInfo,
    Tool, ToolAnnotations,
};
use rmcp::serde_json::Value;
use rmcp::{ErrorData, ServerHandler, tool_handler};
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct DynamicMCP {
    tool_router: ToolRouter<DynamicMCP>,
    instruction: Option<String>,
    server_info: Option<Implementation>,
    server_capabilities: Option<ServerCapabilities>,
}

lazy_static! {
    static ref BODY_ESCAPE_BRACKET_REGEX: Regex =
        Regex::new(r"(\{\s*input\.\w+\s*})|(\{)").unwrap();
}

impl DynamicMCP {
    const URL_TEMPLATE_NAME: &'static str = "url";
    const BODY_TEMPLATE_NAME: &'static str = "body";
    const INPUT_NAME: &'static str = "input";

    pub fn new(config: DynamicMCPConfig) -> Self {
        Self {
            tool_router: Self::tool_router(config.tools),
            instruction: config.instruction,
            server_info: config.server_info,
            server_capabilities: config.server_capabilities,
        }
    }

    fn generate_tool_description(
        description: String,
        name: String,
        input_schema: JsonObject,
        output_schema: Option<JsonObject>,
        annotations: Option<ToolAnnotations>,
    ) -> Tool {
        Tool {
            name: name.into(),
            description: Some(description.into()),
            input_schema: Arc::new(input_schema),
            output_schema: output_schema.map(|schema| Arc::new(schema)),
            annotations,
        }
    }

    fn http_header_template_name(name: String) -> String {
        format!("header_{}", name)
    }

    fn sanitize_http_body_template(body_template: &str) -> String {
        // Use a closure with `replace_all` for conditional replacement
        let modified_string =
            BODY_ESCAPE_BRACKET_REGEX.replace_all(body_template, |caps: &Captures| {
                // Check if the second group (the standalone '{') was captured
                if caps.get(2).is_some() {
                    // If yes, replace it with '\{'
                    "\\{".to_string()
                } else {
                    // Otherwise, it's a template variable (group 1).
                    // Return the original matched string to leave it unchanged.
                    caps[0].to_string()
                }
            });

        modified_string.to_string()
    }

    fn general_http_method_template(
        method: HttpMethod,
        url: String,
        body_template: Option<String>,
        header_template: Option<HashMap<String, String>>,
    ) -> impl Fn(Parameters<Value>) -> BoxFuture<'static, Result<CallToolResult, ErrorData>> {
        // Initialize template once when the function is called
        let mut template = Template::new();
        template
            .add_template(Self::URL_TEMPLATE_NAME, &url)
            .expect("Error registering url template");

        let body_exist = if let Some(ref body_str) = body_template {
            template
                .add_template(
                    Self::BODY_TEMPLATE_NAME,
                    &Self::sanitize_http_body_template(body_str),
                )
                .expect("Error registering body template");
            true
        } else {
            false
        };

        let header_template = header_template.unwrap_or(HashMap::new());

        // Prepare header templates
        let header_template_names: HashMap<String, String> = header_template
            .keys()
            .map(|name| (name.clone(), Self::http_header_template_name(name.clone())))
            .collect();

        // Register header templates
        for (header_name, template_name) in header_template_names.iter() {
            if let Some(header_value) = header_template.get(header_name) {
                if let Err(err) = template.add_template(template_name, header_value) {
                    panic!(
                        "Error while registering header template '{}': {}",
                        header_name, err
                    );
                }
            }
        }

        // Move the initialized template and other data into the closure
        move |Parameters(object): Parameters<Value>| -> BoxFuture<'static, Result<CallToolResult, ErrorData>> {
            // Clone all the captured variables for use in the async block
            let method = method.clone();
            let template = template.clone(); // Clone the pre-initialized template
            let header_template_names = header_template_names.clone();

            Box::pin(async move {
                let context = json!({
                    Self::INPUT_NAME: object
                });

                // Render all templates using the pre-initialized template
                let (rendered_url, rendered_body, rendered_headers) = {
                    // Render URL
                    let rendered_url = template.render(Self::URL_TEMPLATE_NAME, &context).unwrap();

                    // Render body if exists
                    let rendered_body = if body_exist {
                        Some(template.render(Self::BODY_TEMPLATE_NAME, &context).unwrap())
                    } else {
                        None
                    };

                    // Render headers
                    let mut headers = reqwest::header::HeaderMap::new();
                    for (name, template_name) in header_template_names.iter() {
                        let rendered_value = template.render(template_name, &context).unwrap();
                        let header_name = reqwest::header::HeaderName::from_str(name).unwrap();
                        let header_value = reqwest::header::HeaderValue::from_str(&rendered_value).unwrap();
                        headers.insert(header_name, header_value);
                    }

                    (rendered_url, rendered_body, headers)
                };

                // Now build the request without holding the template
                let client = reqwest::Client::new();
                let mut req = match method {
                    HttpMethod::GET => client.get(rendered_url.clone()),
                    HttpMethod::POST => client.post(rendered_url.clone()),
                    HttpMethod::PUT => client.put(rendered_url.clone()),
                    HttpMethod::DELETE => client.delete(rendered_url.clone()),
                };

                if let Some(body) = rendered_body {
                    req = req.body(Body::from(body));
                }

                req = req.headers(rendered_headers);

                let res = req.send().await.map_err(|err| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while sending a request to {}: {}", rendered_url, err),
                        None,
                    )
                })?;

                let response_status = res.status();

                let res_val = res.json::<Value>().await.map_err(|err| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while reading response: {}", err),
                        None,
                    )
                })?;

                match response_status.as_u16() {
                    200..299 => (),
                    _ => return Err(ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while sending a request to {}, got status code : {}, response body : {}", rendered_url, response_status.as_u16(), res_val.to_string()),
                        None,
                    ))
                }

                let content = Content::json::<Value>(res_val).map_err(|err| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while reading content as json: {}", err),
                        None,
                    )
                })?;

                Ok(CallToolResult::success(vec![content]))
            })
        }
    }

    pub fn tool_router(tool_data: Vec<ToolData>) -> ToolRouter<DynamicMCP> {
        let mut router = ToolRouter::new();

        for entry in tool_data.iter() {
            match entry.tool_type {
                ToolType::HTTP => {
                    let Some(ref http_metadata) = entry.http_metadata else {
                        continue;
                    };
                    let method = http_metadata.method.clone();
                    let url = http_metadata.url.clone();
                    let body_template = http_metadata.body_template.clone();
                    let headers = http_metadata.headers.clone();

                    let closure =
                        Self::general_http_method_template(method, url, body_template, headers);
                    let function_tool = DynamicMCPClosure::new(closure);

                    let tool_description = Self::generate_tool_description(
                        entry.description.clone(),
                        entry.name.clone(),
                        http_metadata.input_schema.clone(),
                        http_metadata.output_schema.clone(),
                        entry.tool_annotations.clone(),
                    );

                    router = router.with_route(ToolRoute::new(tool_description, function_tool));
                }
            };
        }

        router
    }
}

#[tool_handler]
impl ServerHandler for DynamicMCP {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: self.instruction.clone(),
            capabilities: self
                .server_capabilities
                .clone()
                .unwrap_or_else(|| ServerCapabilities::builder().enable_tools().build()),
            server_info: self.server_info.clone().unwrap_or_default(),
            ..Default::default()
        }
    }
}
