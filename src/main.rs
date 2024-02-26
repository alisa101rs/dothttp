use std::{
    io::{stderr, stdout},
    path::PathBuf,
};

use clap::{Parser, ValueEnum};
use color_eyre::Result;
use dothttp::{
    output::{parse_format, print::FormattedOutput, CiOutput, Output},
    source::FilesSourceProvider,
    ClientConfig, EnvironmentFileProvider, Runtime,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// A file containing a JSON object that describes the initial values for variables
    #[arg(short = 'n', long)]
    environment_file: Option<PathBuf>,

    /// A file containing a JSON object that persists variables between each invocation
    #[arg(short = 'p', long)]
    snapshot: Option<PathBuf>,

    /// The key value to use on the environment file
    #[arg(short, long)]
    environment: Option<String>,

    files: Vec<String>,

    /// The format of the request output. Only relevant if `-format=standard`.
    ///
    /// Possible values:
    ///
    /// * %R - HTTP protocol
    ///
    /// * %N - Request Name
    ///
    /// * %B - Request Body
    ///
    /// * %H - Request Headers
    #[arg(long, default_value = "%N\n%R\n\n")]
    request_format: String,

    /// The format of the response output. Only relevant if `-format=standard`.
    ///
    /// Possible values:
    ///
    /// * %R - HTTP protocol
    ///
    /// * %T - Response unit tests
    ///
    /// * %B - Response Body
    ///
    /// * %H - Response Headers
    #[arg(long, default_value = "%R\n%H\n%B\n\n%T\n")]
    response_format: String,

    #[arg(long = "accept-invalid-certs")]
    accept_invalid_cert: bool,

    /// Which mode to use to print result. Possible values:
    ///
    /// * standard [default]
    ///
    /// * ci
    #[arg(long = "format", default_value = "standard")]
    format: FormatType,
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

    let Args {
        environment_file,
        snapshot,
        environment,
        files,
        accept_invalid_cert,
        request_format,
        response_format,
        format: output,
    } = Args::parse();

    let env = environment.unwrap_or("dev".to_owned());
    let env_file = environment_file.unwrap_or_else(|| "http-client.env.json".into());
    let snapshot_file = snapshot.unwrap_or_else(|| ".snapshot.json".into());
    let ignore_certificates: bool = accept_invalid_cert;

    let client_config = ClientConfig::new(!ignore_certificates);

    let mut output = get_output(output, request_format, response_format)?;
    let mut environment = EnvironmentFileProvider::open(&env, &env_file, &snapshot_file)?;

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
