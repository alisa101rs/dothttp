use std::{borrow::BorrowMut, io::stdout, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use dothttp::{
    output::{parse_format, print::FormattedOutput},
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

    files: Vec<PathBuf>,

    /// Specific request number to run
    #[arg(short, long)]
    request: Option<usize>,

    /// The format of the request output
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

    /// The format of the response output
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
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let Args {
        environment_file,
        snapshot,
        environment,
        files,
        request,
        accept_invalid_cert,
        request_format,
        response_format,
    } = Args::parse();

    let env = environment.unwrap_or("dev".to_owned());
    let env_file = environment_file.unwrap_or_else(|| "http-client.env.json".into());
    let snapshot_file = snapshot.unwrap_or_else(|| ".snapshot.json".into());
    let ignore_certificates: bool = accept_invalid_cert;

    let client_config = ClientConfig::new(!ignore_certificates);

    let mut stdout = stdout();
    let mut output = FormattedOutput::new(
        stdout.borrow_mut(),
        parse_format(&preprocess_format_strings(request_format))?,
        parse_format(&preprocess_format_strings(response_format))?,
    );
    let mut environment = EnvironmentFileProvider::open(&env, &env_file, &snapshot_file)?;

    let mut runtime = Runtime::new(&mut environment, output.borrow_mut(), client_config).unwrap();

    runtime.execute(files.into_iter(), request).await
}

fn preprocess_format_strings(format: String) -> String {
    format.replace(r"\n", "\n").replace(r"\t", "\t")
}
