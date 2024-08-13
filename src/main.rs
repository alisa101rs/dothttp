use std::{
    io::{stderr, stdout},
    path::PathBuf,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use color_eyre::Result;
use dothttp::{
    output::{parse_format, print::FormattedOutput, CiOutput, Output},
    source::FilesSourceProvider,
    ClientConfig, EnvironmentFileProvider, Runtime,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct CliArgs {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    exec: ExecuteArgs,

    #[command(flatten)]
    env: EnvironmentArgs,
}

#[derive(Debug, Subcommand)]
enum Command {
    Execute {
        #[command(flatten)]
        exec: ExecuteArgs,
        #[command(flatten)]
        env: EnvironmentArgs,
    },
}

#[derive(Debug, Args)]
struct EnvironmentArgs {
    /// A file containing a JSON object that describes the initial values for variables
    #[arg(short = 'n', long)]
    environment_file: Option<PathBuf>,

    /// A file containing a JSON object that persists variables between each invocation
    #[arg(short = 'p', long)]
    snapshot: Option<PathBuf>,

    /// The key value to use on the environment file
    #[arg(short, long)]
    environment: Option<String>,
}

#[derive(Debug, Args)]
struct ExecuteArgs {
    /// The format of the request output. Only relevant if `-format=standard`.
    ///
    /// [possible values:
    /// %R - HTTP protocol,
    /// %N - request Name,
    /// %B - request Body,
    /// %H - request Headers]
    #[arg(long, default_value = "%N\n%R\n\n")]
    request_format: String,

    /// The format of the response output. Only relevant if `-format=standard`.
    ///
    /// [possible values:
    /// %R - HTTP protocol,
    /// %T - Response unit tests,
    /// %B - Response Body,
    /// %H - Response Headers]
    #[arg(long, default_value = "%R\n%H\n%B\n\n%T\n")]
    response_format: String,

    #[arg(long = "accept-invalid-certs")]
    accept_invalid_cert: bool,

    /// Which mode to use to print result.
    #[arg(long = "format", default_value = "standard")]
    format: FormatType,

    files: Vec<String>,
}

#[derive(Debug, Default, Copy, Clone, ValueEnum)]
enum FormatType {
    #[default]
    Standard,
    Ci,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<std::process::ExitCode> {
    color_eyre::install()?;

    let CliArgs { command, exec, env } = CliArgs::parse();

    let command = command.unwrap_or(Command::Execute { exec, env });

    match command {
        Command::Execute { exec, env } => run_execute(environment(env)?, exec).await,
    }
}

fn environment(
    EnvironmentArgs {
        environment_file,
        snapshot,
        environment,
    }: EnvironmentArgs,
) -> Result<EnvironmentFileProvider> {
    let env = environment.unwrap_or("dev".to_owned());
    let env_file = environment_file.unwrap_or_else(|| "http-client.env.json".into());
    let snapshot_file = snapshot.unwrap_or_else(|| ".snapshot.json".into());
    EnvironmentFileProvider::open(&env, &env_file, &snapshot_file)
}

async fn run_execute(
    mut environment: EnvironmentFileProvider,
    args: ExecuteArgs,
) -> Result<std::process::ExitCode> {
    let ExecuteArgs {
        request_format,
        response_format,
        accept_invalid_cert,
        format,
        files,
    } = args;

    let ignore_certificates: bool = accept_invalid_cert;

    let client_config = ClientConfig::new(!ignore_certificates);

    let mut output = get_output(format, request_format, response_format)?;

    let mut runtime = Runtime::new(&mut environment, &mut output, client_config).unwrap();

    runtime
        .execute(FilesSourceProvider::from_list(&files)?)
        .await?;

    Ok(output.exit_code())
}

fn get_output(
    ty: FormatType,
    request_format: String,
    response_format: String,
) -> Result<Box<dyn Output>> {
    Ok(match ty {
        FormatType::Standard => Box::new(FormattedOutput::new(
            stdout(),
            stderr(),
            parse_format(&preprocess_format_strings(request_format))?,
            parse_format(&preprocess_format_strings(response_format))?,
        )),
        FormatType::Ci => Box::new(CiOutput::default()),
    })
}

fn preprocess_format_strings(format: String) -> String {
    format.replace(r"\n", "\n").replace(r"\t", "\t")
}
