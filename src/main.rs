mod core;

use clap::Parser;
use core::config::{DynamicMCPConfig, TransportType};
use rmcp::ServiceExt;
use rmcp::transport::stdio;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short = 'f', long = "file_path", help = "File path to the yaml file")]
    file_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config = DynamicMCPConfig::new_from_file(args.file_path.clone()).await;

    let service = core::engine::DynamicMCP::new(config.clone());

    let transport_config = config.transport_config.unwrap_or_default();

    match transport_config.transport_type {
        TransportType::STDIO => {
            let service = service.serve(stdio()).await.inspect_err(|err| {
                eprintln!("Error while starting the service: {}", err);
            })?;

            service.waiting().await?;
            Ok(())
        }
        _ => {
            panic!("Unsupported transport type");
        }
    }
}
