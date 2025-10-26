# EasyMCP

[![CI](https://github.com/hsjsjsj009/EasyMCP/workflows/CI-PR-Checks/badge.svg)](https://github.com/hsjsjsj009/EasyMCP/actions/workflows/ci.yml)
[![Release](https://github.com/hsjsjsj009/EasyMCP/workflows/Release/badge.svg)](https://github.com/hsjsjsj009/EasyMCP/actions/workflows/release.yml)
[![Code Quality](https://github.com/hsjsjsj009/EasyMCP/workflows/Code%20Quality/badge.svg)](https://github.com/hsjsjsj009/EasyMCP/actions/workflows/quality.yml)

An Easy Model Context Protocol (MCP) server that allows you to define and execute tools via HTTP requests or command execution through a simple YAML configuration file.

## Description

EasyMCP is a flexible MCP server implementation in Rust that enables you to create custom tools without writing code. It supports two transport mechanisms:

- **STDIO**: Standard input/output communication for local tool execution
- **SSE (Server-Sent Events)**: HTTP-based server for remote tool execution with real-time communication

The server can execute tools in two ways:
- **HTTP Tools**: Make HTTP requests (GET, POST, PUT, DELETE) with templated URLs, headers, and request bodies
- **Command Tools**: Execute system commands with templated arguments and stdin input

## How To Use



### Installation

#### Build from source

##### Prerequisites

Make sure you have Rust installed on your system:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

1. Clone the repository:
```bash
git clone <repository-url>
cd EasyMCP
```

2. Build the project:
```bash
cargo build --release
```

#### Install from release

You can download the pre-built binary from the [releases page](https://github.com/hsjsjsj009/EasyMCP/releases).

### Running the Server

The CLI takes a single argument: the path to a YAML configuration file.

```bash
# For STDIO transport
./target/release/easymcp --file_path example/mcp.yaml

# For SSE transport
./target/release/easymcp --file_path example/mcp-sse.yaml
```

#### STDIO Mode

In STDIO mode, the server communicates through standard input/output, making it suitable for integration with MCP clients that support this transport mechanism.

#### SSE Mode

In SSE mode, the server starts an HTTP server that listens for MCP requests via Server-Sent Events. The server will display the listening address when started.

### Example Usage

The repository includes example configurations:

- `example/mcp.yaml` - STDIO transport example with weather forecast tools
- `example/mcp-sse.yaml` - SSE transport example with weather forecast tools
- `example/input.sh` - Example script used by command tools

## Development & CI/CD

This project includes comprehensive GitHub Actions workflows for automated testing, building, and releasing:

### Workflows

- **CI-PR-Checks** (`.github/workflows/ci.yml`): Runs on pushes and pull requests
  - Tests the code on Ubuntu Linux
  - Builds binaries for multiple platforms (Linux x86_64/ARM64, Windows x86_64, macOS x86_64/ARM64)
  - Uploads build artifacts for testing

- **Release** (`.github/workflows/release.yml`): Runs when tags are pushed (e.g., `v1.0.0`)
  - Builds release binaries for all supported platforms
  - Creates GitHub releases with downloadable binaries
  - Automatically generates release notes

- **Code Quality** (`.github/workflows/quality.yml`): Ensures code standards
  - Runs `cargo fmt --check` for formatting validation
  - Runs `cargo clippy` for linting
  - Runs `cargo check` for compilation validation

### Supported Platforms

The CI builds binaries for the following platforms:
- **Linux**: x86_64 and ARM64
- **Windows**: x86_64
- **macOS**: x86_64 (Intel) and ARM64 (Apple Silicon)

### Building Locally

For development and testing, you can also build for specific targets:

```bash
# Build for your current platform
cargo build --release

# Cross-compile for Linux ARM64
cargo build --release --target aarch64-unknown-linux-gnu

# Cross-compile for Windows
cargo build --release --target x86_64-pc-windows-msvc

# Cross-compile for macOS ARM64
cargo build --release --target aarch64-apple-darwin
```

Alternatively, you can use the provided Makefile for common development tasks:

```bash
make build    # Build for current platform
make test     # Run tests
make lint     # Run clippy lints
make fmt      # Format code
make clean    # Clean build artifacts
make help     # Show all available commands
```

## Config Details

The configuration is defined in a YAML file with the following structure:

```yaml
# Optional instruction for the MCP server
instruction: "Description of what these tools do"

# Optional server information
server_info:
  name: "My MCP Server"
  version: "1.0.0"

# Transport configuration
transport_config:
  transport_type: STDIO  # or SSE
  sse_config:           # Only required for SSE transport
    address: "127.0.0.1:8080"
    sse_path: "/sse"           # Optional, defaults to "/sse"
    post_path: "/message"      # Optional, defaults to "/message"
    keep_alive_duration: "5s"  # Optional keep-alive duration

# Array of tools
tools:
  - name: "tool_name"
    description: "Description of what this tool does"
    tool_type: HTTP  # or COMMAND
    # For HTTP tools
    http_metadata:
      url: "https://api.example.com/data?param={ input.parameter }"
      method: GET  # GET, POST, PUT, DELETE
      body: |      # Optional request body template
        {
          "data": "{ input.data }"
        }
      headers:     # Optional headers map
        Authorization: "Bearer { input.token }"
        Content-Type: "application/json"
      input_schema:    # JSON schema for input validation
        type: object
        properties:
          parameter:
            type: string
            description: "Input parameter"
          data: 
            type: string
          token:
            type: string
        required: ["parameter", "token"]
      output_schema:   # Optional JSON schema for output validation
        type: object
        properties:
          result:
            type: string
    # For COMMAND tools
    command_metadata:
      command: "bash"
      args:         # Optional array of command arguments
        - "./script.sh"
        - "{ input.arg1 }"
        - "{ input.arg2 }"
      stdin: "Input data: { input.data }"  # Optional stdin template
      input_schema:
        type: object
        properties:
          arg1:
            type: string
          arg2:
            type: number
          data:
            type: string
      output_schema:
        type: object
        properties:
          output:
            type: string
```

### Template Variables

The configuration supports template variables using the `{ input.field }` syntax. We only provide `input` as the context. You can see the example below:

- In HTTP URLs: `https://api.example.com/users/{ input.user_id }`
- In request bodies: `{"user": "{ input.username }"}`
- In headers: `Authorization: "Bearer { input.token }"`
- In command arguments: `["--user", "{ input.username }"]`
- In stdin: `echo "Processing { input.filename }"`

We use TinyTemplate for template rendering engine. It is a simple and fast template rendering engine. For more details, please refer to [TinyTemplate Documentation](https://docs.rs/tinytemplate/latest/tinytemplate/).

### Custom Template Formatters

In addition to the standard template variables, EasyMCP provides custom formatters for common use cases:

#### url_encode Formatter

The `url_encode` formatter URL-encodes JSON values, which is useful when you need to include data in URLs or query parameters that might contain special characters.

**Usage:**
```yaml
http_metadata:
  url: "https://api.example.com/search?q={ input.query | url_encode }"
  method: GET
```

**Example:**
- Input: `{"query": "hello world & more"}`
- Template: `{ input.query | url_encode }`
- Output: `hello%20world%20%26%20more`

The formatter works by converting the JSON value to a string and then URL-encoding it using the `urlencoding` crate. This ensures that special characters like spaces, ampersands, and other URL-unsafe characters are properly encoded for use in HTTP requests.

### Input/Output Schemas

Both HTTP and COMMAND tools support JSON Schema for input and output validation:

```yaml
input_schema:
  type: object
  properties:
    latitude:
      type: number
      description: "Latitude of the location"
    longitude:
      type: number
      description: "Longitude of the location"
  required: ["latitude", "longitude"]

output_schema:
  type: object
  properties:
    temperature:
      type: number
      description: "Temperature in Celsius"
    humidity:
      type: number
      description: "Relative humidity percentage"
```

### Transport Types

#### STDIO Transport
- Simple configuration with `transport_type: STDIO`
- No additional transport configuration needed
- Suitable for local development and testing

#### SSE Transport
- Requires `transport_type: SSE`
- Must include `sse_config` section
- Starts HTTP server with configurable endpoints
- Supports real-time communication via Server-Sent Events

### Tool Types

#### HTTP Tools
- Make HTTP requests to external APIs
- Support all HTTP methods (GET, POST, PUT, DELETE)
- Template support for URLs, headers, and request bodies
- Automatic JSON parsing for responses

#### Command Tools
- Execute system commands
- Template support for command arguments and stdin
- Capture stdout/stderr as tool output
- Support for JSON output parsing

### Error Handling

The server provides detailed error messages for:
- Invalid configuration files
- Template rendering errors
- HTTP request failures
- Command execution failures
- JSON parsing errors

### Security Considerations

- Be cautious with command execution tools
- Validate and sanitize all inputs
- Use HTTPS for HTTP tools when possible
- Consider authentication for SSE servers in production

### Testing MCP

You can test the MCP server using model context protocol inspector. Below is the example of the command
```bash
# SSE
npx @modelcontextprotocol/inspector http://127.0.0.1:8080

# STDIO
npx @modelcontextprotocol/inspector <path-to-binary> --file_path <path-to-config-file>
```

Before using `npx`, you need to download and install `nodejs` distribution.
