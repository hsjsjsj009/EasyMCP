mod core;

use clap::Parser;
use core::config::{DynamicMCPConfig, TransportType};
use duration_string::DurationString;
use rmcp::ServiceExt;
use rmcp::transport::sse_server::SseServerConfig;
use rmcp::transport::{SseServer, stdio};
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short = 'f', long = "file_path", help = "File path to the yaml file")]
    file_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config = DynamicMCPConfig::new_from_file(args.file_path.clone()).await;

    let Some(ref transport_config) = config.transport_config else {
        panic!("transport_config is required");
    };

    match transport_config.transport_type {
        TransportType::STDIO => {
            let service = core::engine::DynamicMCP::new(config.clone());

            let service = service.serve(stdio()).await.inspect_err(|err| {
                panic!("Error while starting the service: {}", err);
            })?;

            service.waiting().await?;
        }
        TransportType::SSE => {
            let Some(ref sse_config) = transport_config.sse_config else {
                panic!("sse_config is required");
            };

            let sse_server_config = SseServerConfig {
                bind: sse_config.address.parse()?,
                sse_path: sse_config.sse_path.clone().unwrap_or("/sse".to_string()),
                post_path: sse_config
                    .post_path
                    .clone()
                    .unwrap_or("/message".to_string()),
                sse_keep_alive: sse_config
                    .keep_alive_duration
                    .clone()
                    .map(|val| DurationString::from_string(val).unwrap().into()),
                ct: CancellationToken::new(),
            };

            let (sse_server, router) = SseServer::new(sse_server_config);

            let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;

            let ct = sse_server.config.ct.child_token();

            let server = axum::serve(listener, router).with_graceful_shutdown(async move {
                ct.cancelled().await;
                tracing::info!("sse server cancelled");
            });

            tokio::spawn(async move {
                if let Err(e) = server.await {
                    tracing::error!(error = %e, "sse server shutdown with error");
                }
            });

            println!("Server listening on {}", sse_server.config.bind);

            let ct = sse_server.with_service(move || core::engine::DynamicMCP::new(config.clone()));

            tokio::signal::ctrl_c().await?;
            ct.cancel();
        }
    }

    Ok(())
}
