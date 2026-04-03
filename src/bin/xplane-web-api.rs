use std::num::NonZeroU64;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;
use xplane_web_api::error::RestClientError;
use xplane_web_api::rest::types::{
    ActivateCommandRequest, DatarefValueWriteRequest, FlightRequest,
};
use xplane_web_api::rest::{self, Client, DEFAULT_REST_API_BASE_URL, ResponseValue};

#[derive(Debug, Error)]
enum CliError {
    #[error("Failed to initialize logger: {message}")]
    LoggerInit { message: String },

    #[error("--base-url must not be empty")]
    EmptyBaseUrl,

    #[error("Missing {context} body. Provide --json-body or --json-body-file")]
    MissingJsonBody { context: &'static str },

    #[error("Conflicting {context} body inputs. Provide either --json-body or --json-body-file")]
    ConflictingJsonBodyInputs { context: &'static str },

    #[error("Failed to read JSON body file `{path}`: {source}")]
    ReadJsonBodyFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse {context} JSON request body: {source}")]
    ParseJsonBody {
        context: &'static str,
        #[source]
        source: serde_json::Error,
    },

    #[error("Failed to serialize response: {source}")]
    SerializeResponse {
        #[source]
        source: serde_json::Error,
    },

    #[error(transparent)]
    Rest(#[from] RestClientError),
}

#[derive(Parser, Debug)]
#[command(name = "xplane-web-api")]
#[command(about = "CLI for the X-Plane REST API client")]
struct Cli {
    #[arg(long, global = true, default_value = DEFAULT_REST_API_BASE_URL)]
    base_url: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    GetCapabilities,
    ListDatarefs(ListRefsArgs),
    GetDatarefCount,
    GetDatarefValue(GetDatarefValueArgs),
    SetDatarefValue(SetDatarefValueArgs),
    ListCommands(ListRefsArgs),
    GetCommandCount,
    #[allow(clippy::enum_variant_names)]
    ActivateCommand(ActivateCommandArgs),
    StartFlight(FlightMutationArgs),
    UpdateFlight(FlightMutationArgs),
}

#[derive(Args, Debug)]
struct ListRefsArgs {
    #[arg(long)]
    fields: Option<String>,
    #[arg(long = "filter-name")]
    filter_name: Vec<String>,
    #[arg(long)]
    limit: Option<NonZeroU64>,
    #[arg(long)]
    start: Option<i64>,
}

#[derive(Args, Debug)]
struct GetDatarefValueArgs {
    #[arg(long)]
    id: i64,
    #[arg(long)]
    index: Option<i64>,
}

#[derive(Args, Debug)]
struct SetDatarefValueArgs {
    #[arg(long)]
    id: i64,
    #[arg(long)]
    index: Option<i64>,
    #[command(flatten)]
    body: JsonBodyInput,
}

#[derive(Args, Debug)]
struct ActivateCommandArgs {
    #[arg(long)]
    id: i64,
    #[arg(long, conflicts_with_all = ["json_body", "json_body_file"])]
    duration: Option<f64>,
    #[command(flatten)]
    body: JsonBodyInput,
}

#[derive(Args, Debug)]
struct FlightMutationArgs {
    #[command(flatten)]
    body: JsonBodyInput,
}

#[derive(Args, Debug)]
struct JsonBodyInput {
    #[arg(
        long = "json-body",
        value_name = "JSON",
        conflicts_with = "json_body_file"
    )]
    json_body: Option<String>,
    #[arg(
        long = "json-body-file",
        value_name = "PATH",
        conflicts_with = "json_body"
    )]
    json_body_file: Option<PathBuf>,
}

fn parse_required_body<T>(input: &JsonBodyInput, context: &'static str) -> Result<T, CliError>
where
    T: DeserializeOwned,
{
    match (&input.json_body, &input.json_body_file) {
        (Some(json_body), None) => serde_json::from_str(json_body)
            .map_err(|source| CliError::ParseJsonBody { context, source }),
        (None, Some(path)) => {
            let json_text =
                std::fs::read_to_string(path).map_err(|source| CliError::ReadJsonBodyFile {
                    path: path.clone(),
                    source,
                })?;
            serde_json::from_str(&json_text)
                .map_err(|source| CliError::ParseJsonBody { context, source })
        }
        (None, None) => Err(CliError::MissingJsonBody { context }),
        (Some(_), Some(_)) => Err(CliError::ConflictingJsonBodyInputs { context }),
    }
}

fn print_success<T>(response: ResponseValue<T>) -> Result<(), CliError>
where
    T: Serialize,
{
    let output = serde_json::to_string_pretty(response.as_ref())
        .map_err(|source| CliError::SerializeResponse { source })?;
    println!("{output}");
    Ok(())
}

fn print_success_no_body(response: ResponseValue<()>) {
    println!("{}", response.status());
}

fn rest_error<T>(source: rest::Error<T>) -> CliError
where
    RestClientError: From<rest::Error<T>>,
{
    CliError::Rest(source.into())
}

async fn run(cli: Cli) -> Result<(), CliError> {
    let client = Client::new(&cli.base_url);

    match cli.command {
        Command::GetCapabilities => {
            let response = client.get_capabilities().await.map_err(rest_error)?;
            print_success(response)
        }
        Command::ListDatarefs(args) => {
            let filter_name = if args.filter_name.is_empty() {
                None
            } else {
                Some(&args.filter_name)
            };
            let response = client
                .list_datarefs(args.fields.as_deref(), filter_name, args.limit, args.start)
                .await
                .map_err(rest_error)?;
            print_success(response)
        }
        Command::GetDatarefCount => {
            let response = client.get_dataref_count().await.map_err(rest_error)?;
            print_success(response)
        }
        Command::GetDatarefValue(args) => {
            let response = client
                .get_dataref_value(args.id, args.index)
                .await
                .map_err(rest_error)?;
            print_success(response)
        }
        Command::SetDatarefValue(args) => {
            let body: DatarefValueWriteRequest = parse_required_body(&args.body, "set-dataref")?;
            let response = client
                .set_dataref_value(args.id, args.index, &body)
                .await
                .map_err(rest_error)?;
            print_success_no_body(response);
            Ok(())
        }
        Command::ListCommands(args) => {
            let filter_name = if args.filter_name.is_empty() {
                None
            } else {
                Some(&args.filter_name)
            };
            let response = client
                .list_commands(args.fields.as_deref(), filter_name, args.limit, args.start)
                .await
                .map_err(rest_error)?;
            print_success(response)
        }
        Command::GetCommandCount => {
            let response = client.get_command_count().await.map_err(rest_error)?;
            print_success(response)
        }
        Command::ActivateCommand(args) => {
            let body = if let Some(duration) = args.duration {
                ActivateCommandRequest { duration }
            } else {
                parse_required_body(&args.body, "activate-command")?
            };
            let response = client
                .activate_command(args.id, &body)
                .await
                .map_err(rest_error)?;
            print_success_no_body(response);
            Ok(())
        }
        Command::StartFlight(args) => {
            let body: FlightRequest = parse_required_body(&args.body, "start-flight")?;
            let response = client.start_flight(&body).await.map_err(rest_error)?;
            print_success_no_body(response);
            Ok(())
        }
        Command::UpdateFlight(args) => {
            let body: FlightRequest = parse_required_body(&args.body, "update-flight")?;
            let response = client.update_flight(&body).await.map_err(rest_error)?;
            print_success_no_body(response);
            Ok(())
        }
    }
}

async fn try_main() -> Result<(), CliError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init()
        .map_err(|error| CliError::LoggerInit {
            message: error.to_string(),
        })?;

    let cli = Cli::parse();
    if cli.base_url.trim().is_empty() {
        return Err(CliError::EmptyBaseUrl);
    }

    run(cli).await
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = try_main().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
