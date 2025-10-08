use crate::core::closure::DynamicMCPClosure;
use crate::core::config::{DynamicMCPConfig, HttpMethod, ToolData, ToolType};
use crate::core::template::Template;
use futures_core::future::BoxFuture;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use reqwest::Body;
use reqwest::header::{CONTENT_TYPE, HeaderValue};
use rmcp::handler::server::tool::{Parameters, ToolRoute, ToolRouter};
use rmcp::model::{
    CallToolResult, Content, ErrorCode, Implementation, JsonObject, ServerCapabilities, ServerInfo,
    Tool, ToolAnnotations,
};
use rmcp::serde_json::Value;
use rmcp::{ErrorData, ServerHandler, tool_handler};
use serde_json::json;
use std::collections::HashMap;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
pub struct DynamicMCP {
    tool_router: ToolRouter<DynamicMCP>,
    instruction: Option<String>,
    server_info: Option<Implementation>,
    server_capabilities: Option<ServerCapabilities>,
}

lazy_static! {
    static ref ESCAPE_BRACKET_REGEX: Regex =
        Regex::new(r"(\{\s*input\.\w+\s*})|(\{)").unwrap(); // This regex is used to escape the brackets in the template. For further details, see https://docs.rs/tinytemplate/latest/tinytemplate/syntax/index.html#escaping-curly-braces
}

impl DynamicMCP {
    const URL_TEMPLATE_NAME: &'static str = "url";
    const BODY_TEMPLATE_NAME: &'static str = "body";
    const INPUT_NAME: &'static str = "input";
    const COMMAND_TEMPLATE_NAME: &'static str = "command";
    const STDIN_TEMPLATE_NAME: &'static str = "stdin";

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

    fn command_args_template_name(idx: usize) -> String {
        format!("args_{}", idx)
    }

    fn sanitize_template_text(body_template: &str) -> String {
        // Use a closure with `replace_all` for conditional replacement
        let modified_string = ESCAPE_BRACKET_REGEX.replace_all(body_template, |caps: &Captures| {
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
        tool_index: usize,
        method: HttpMethod,
        url: String,
        body_template: Option<String>,
        header_template: Option<HashMap<String, String>>,
    ) -> impl Fn(Parameters<Value>) -> BoxFuture<'static, Result<CallToolResult, ErrorData>> {
        // Initialize template once when the function is called
        let mut template = Template::new();
        template
            .add_template(
                Self::URL_TEMPLATE_NAME,
                &Self::sanitize_template_text(url.as_str()),
            )
            .expect(format!("Error registering url template, tool index {}", tool_index).as_str());

        let body_exist = if let Some(ref body_str) = body_template {
            template
                .add_template(
                    Self::BODY_TEMPLATE_NAME,
                    &Self::sanitize_template_text(body_str),
                )
                .expect(
                    format!("Error registering body template, tool index {}", tool_index).as_str(),
                );
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
                template
                    .add_template(template_name, &Self::sanitize_template_text(header_value))
                    .expect(
                        format!(
                            "Error registering header template, tool index {}, header name {}",
                            tool_index, header_name
                        )
                        .as_str(),
                    );
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

                // Render headers
                let mut headers = reqwest::header::HeaderMap::new();
                for (name, template_name) in header_template_names.iter() {
                    let rendered_value = template.render(template_name, &context).map_err(|err| ErrorData::new(
                        ErrorCode::PARSE_ERROR,
                        format!("Error while rendering header template, header name {} : {}", name, err.to_string()),
                        None,
                    ))?;
                    let header_name = reqwest::header::HeaderName::from_str(name).unwrap();
                    let header_value = reqwest::header::HeaderValue::from_str(&rendered_value).unwrap();
                    headers.insert(header_name, header_value);
                }

                // Render URL
                let rendered_url = template.render(Self::URL_TEMPLATE_NAME, &context).map_err(|err| ErrorData::new(
                    ErrorCode::PARSE_ERROR,
                    format!("Error while rendering url template: {}", err.to_string()),
                    None,
                ))?;

                // Render body if exists
                let rendered_body = if body_exist {
                    let temp = template.render(Self::BODY_TEMPLATE_NAME, &context).map_err(|err| ErrorData::new(
                            ErrorCode::PARSE_ERROR,
                            format!("Error while rendering body template: {}", err.to_string()),
                            None,
                        ))?;
                    Some(temp)
                } else {
                    None
                };

                let rendered_headers = headers;

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


                let response_status = res.status().as_u16();

                let empty_header_value = HeaderValue::from_static("");

                let content_type = res.headers().get(CONTENT_TYPE).unwrap_or(&empty_header_value).to_str().unwrap_or("").to_string();

                let content_length = res.content_length().unwrap_or(0);

                let res_text = res.text().await.map_err(|err| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while reading content from {}: {}", rendered_url, err),
                        None,
                    )
                });

                if res_text.is_err() && content_length > 0 {
                    return Err(res_text.err().unwrap());
                }

                let res_val = res_text.unwrap();

                let res_val = if content_type.contains("application/json") {
                    serde_json::from_str::<Value>(&res_val).map_err(|err| {
                        ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            format!("Error while parsing json content from {}: {}", rendered_url, err),
                            None,
                        )
                    })?
                } else {
                    Value::String(res_val)
                };

                match response_status {
                    200..=299 => (),
                    _ => return Err(ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while sending a request to {}, got status code : {}, response body : {}", rendered_url, response_status, res_val.to_string()),
                        None,
                    ))
                }

                let content = Content::json::<Value>(res_val).map_err(|err| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while parsing content as json: {}", err),
                        None,
                    )
                })?;

                Ok(CallToolResult::success(vec![content]))
            })
        }
    }

    fn general_command_template(
        tool_index: usize,
        command_template: String,
        args_template: Option<Vec<String>>,
        stdin_template: Option<String>,
    ) -> impl Fn(Parameters<Value>) -> BoxFuture<'static, Result<CallToolResult, ErrorData>> {
        // Initialize template once when the function is called
        let mut template = Template::new();
        template
            .add_template(
                Self::COMMAND_TEMPLATE_NAME,
                &Self::sanitize_template_text(command_template.as_str()),
            )
            .expect(
                format!(
                    "Error registering command template, tool index {}: {}",
                    tool_index, command_template
                )
                .as_str(),
            );

        let stdin_template_exist = if let Some(ref stdin_template) = stdin_template {
            template
                .add_template(
                    Self::STDIN_TEMPLATE_NAME,
                    &Self::sanitize_template_text(stdin_template),
                )
                .expect(
                    format!(
                        "Error registering stdin template, tool index {}",
                        tool_index
                    )
                    .as_str(),
                );
            true
        } else {
            false
        };

        let args_template = args_template.unwrap_or(vec![]);
        for (i, args) in args_template.iter().enumerate() {
            let template_name = Self::command_args_template_name(i);
            template
                .add_template(&template_name, &Self::sanitize_template_text(args))
                .expect(
                    format!(
                        "Error registering args template, tool index {}, arg index {}",
                        tool_index, i
                    )
                    .as_str(),
                );
        }

        move |Parameters(object): Parameters<Value>| -> BoxFuture<'static, Result<CallToolResult, ErrorData>> {
            let template = template.clone(); // Clone the pre-initialized template
            let args_template = args_template.clone();

            Box::pin(async move {
                let context = json!({
                    Self::INPUT_NAME: object
                });

                let rendered_command = template.render(Self::COMMAND_TEMPLATE_NAME, &context).map_err(|err| ErrorData::new(
                    ErrorCode::PARSE_ERROR,
                    format!("Error while rendering command template: {}", err.to_string()),
                    None,
                ))?;

                let args_template = args_template.iter().enumerate().map(|(i,_)| template.render(&Self::command_args_template_name(i), &context).map_err(|err| ErrorData::new(
                    ErrorCode::PARSE_ERROR,
                    format!("Error while rendering args template: {}", err.to_string()),
                    None,
                ))).collect::<Result<Vec<String>, ErrorData>>()?;

                let mut command = tokio::process::Command::new(rendered_command);

                if stdin_template_exist {
                    command.stdin(Stdio::piped());
                }

                let mut command = command
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .args(&args_template)
                    .spawn()
                    .map_err(|err| ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while spawning a process: {}", err),
                        None,
                    ))?;

                if stdin_template_exist {
                    let mut stdin = match command.stdin.take() {
                        Some(stdin) => stdin,
                        None => return Err(ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            "Error while spawning a process: stdin is None".to_string(),
                            None
                        ))
                    };

                    let stdin_data = template.render(Self::STDIN_TEMPLATE_NAME, &context).map_err(|err| ErrorData::new(
                        ErrorCode::PARSE_ERROR,
                        format!("Error while rendering stdin template: {}", err.to_string()),
                        None,
                    ))?;
                    stdin.write_all(stdin_data.as_bytes()).await.map_err(|err| ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while writing stdin: {}", err),
                        None
                    ))?;
                }

                let output = command.wait_with_output().await.map_err(|err| ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Error while waiting for a process: {}", err),
                    None,
                ))?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();

                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                if !output.status.success() {
                    return Err(ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Error while executing a command: {}", stderr),
                        None,
                    ))
                }

                if let Ok(json_output) = serde_json::from_str::<Value>(&stdout) {
                    let content = Content::json::<Value>(json_output).map_err(|err| {
                        ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            format!("Error while parsing content as json: {}", err),
                            None,
                        )
                    })?;
                    return Ok(CallToolResult::success(vec![content]));
                }

                Ok(CallToolResult::success(vec![Content::text(stdout)]))
            })

        }
    }

    pub fn tool_router(tool_data: Vec<ToolData>) -> ToolRouter<DynamicMCP> {
        let mut router = ToolRouter::new();

        for (i, entry) in tool_data.iter().enumerate() {
            let (function_tool, tool_description) = match entry.tool_type {
                ToolType::HTTP => {
                    let Some(ref http_metadata) = entry.http_metadata else {
                        continue;
                    };
                    let method = http_metadata.method.clone();
                    let url = http_metadata.url.clone();
                    let body_template = http_metadata.body.clone();
                    let headers = http_metadata.headers.clone();

                    let closure =
                        Self::general_http_method_template(i, method, url, body_template, headers);
                    let function_tool = DynamicMCPClosure::new(closure);

                    let tool_description = Self::generate_tool_description(
                        entry.description.clone(),
                        entry.name.clone(),
                        http_metadata.input_schema.clone(),
                        http_metadata.output_schema.clone(),
                        entry.tool_annotations.clone(),
                    );

                    (function_tool, tool_description)
                }

                ToolType::COMMAND => {
                    let Some(ref command_metadata) = entry.command_metadata else {
                        continue;
                    };
                    let command_template = command_metadata.command.clone();
                    let args_template = command_metadata.args.clone();
                    let stdin_template = command_metadata.stdin.clone();

                    let closure = Self::general_command_template(
                        i,
                        command_template,
                        args_template,
                        stdin_template,
                    );
                    let function_tool = DynamicMCPClosure::new(closure);

                    let tool_description = Self::generate_tool_description(
                        entry.description.clone(),
                        entry.name.clone(),
                        command_metadata.input_schema.clone(),
                        command_metadata.output_schema.clone(),
                        entry.tool_annotations.clone(),
                    );

                    (function_tool, tool_description)
                }
            };

            router = router.with_route(ToolRoute::new(tool_description, function_tool));
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
