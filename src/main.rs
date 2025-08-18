use rmcp::handler::server::tool::{cached_schema_for_type, Parameters, ToolRouter};
use rmcp::model::{CallToolResult, Content, JsonObject, ServerCapabilities, ServerInfo, Tool};
use rmcp::{tool_handler, ErrorData, ServerHandler, ServiceExt};
use rmcp::serde_json::{Value};
use rmcp::transport::stdio;
use serde;
use schemars;

pub struct DynamicMCP {
    tool_router: ToolRouter<DynamicMCP>
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DynamicMCPGeneralRequest {
    pub input: JsonObject
}

impl DynamicMCP {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router()
        }
    }

    pub fn tool_router() -> ToolRouter<DynamicMCP> {
        ToolRouter::new()
            .with_route((Tool{
                name: "test".into(),
                description: Some("Get weather forecast for a location.\nArgs:\nlatitude: Latitude of the location\nlongitude: Longitude of the location".into()),
                input_schema: cached_schema_for_type::<Parameters<DynamicMCPGeneralRequest>>(),
                output_schema: None,
                annotations: None
            }, |Parameters(object): Parameters<DynamicMCPGeneralRequest>| -> Result<CallToolResult, ErrorData> {
                Ok(CallToolResult::success(vec![Content::text(
                    Value::Object(object.input).to_string()
                )]))
            }))
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


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = DynamicMCP::new().serve(stdio()).await.inspect_err(|err| eprintln!("{:?}", err))?;

    service.waiting().await?;

    Ok(())
}
