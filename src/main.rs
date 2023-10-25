use std::{borrow::BorrowMut, io::stdout, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use dot_http::{
    output::{parse_format, print::FormattedOutput},
    ClientConfig, Runtime,
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

    file: PathBuf,

    /// Specific request number to run
    #[arg(short, long)]
    request: Option<usize>,

    #[arg(long = "danger-accept-invalid-certs")]
    accept_invalid_cert: bool,
}

fn main() -> Result<()> {
    let Args {
        environment_file,
        snapshot,
        environment,
        file,
        request,
        accept_invalid_cert,
    } = Args::parse();

    let script_file = file;
    let env = environment.unwrap_or("dev".to_owned());
    let env_file = environment_file.unwrap_or_else(|| "./env.json".into());
    let snapshot_file = snapshot.unwrap_or_else(|| ".snapshot.json".into());
    let ignore_certificates: bool = accept_invalid_cert;
    let response_format = "%R\n%H\n%B\n";
    let request_format = "%R\n\n";

    let client_config = ClientConfig::new(!ignore_certificates);

    let mut stdout = stdout();
    let mut output = FormattedOutput::new(
        stdout.borrow_mut(),
        parse_format(request_format)?,
        parse_format(response_format)?,
    );

    let mut runtime = Runtime::new(
        &env,
        &snapshot_file,
        &env_file,
        output.borrow_mut(),
        client_config,
    )
    .unwrap();

    runtime.execute(&script_file, request)
}
