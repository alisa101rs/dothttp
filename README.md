## **This is a fork of existing tool [dot-http](https://github.com/bayne/dot-http)**

# dothttp

[![codecov](https://codecov.io/gh/alisa101rs/dothttp/graph/badge.svg?token=9GWCS5H23D)](https://codecov.io/gh/alisa101rs/dothttp)
[![Crates.io](https://img.shields.io/crates/v/dothttp.svg)](https://crates.io/crates/dothttp)
[![build](https://github.com/alisa101rs/dothttp/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/alisa101rs/dothttp/actions/workflows/test.yml)
[![nix](https://github.com/alisa101rs/dothttp/actions/workflows/nix.yml/badge.svg?branch=main)](https://github.com/alisa101rs/dothttp/actions/workflows/nix.yml)

`dothttp` is a text-based scriptable HTTP client.
It is a simple language that resembles the actual HTTP protocol but with just a smidgen of magic to make it more practical for someone who builds and tests APIs.

The difference from [dot-http](https://github.com/bayne/dot-http) is that `dothttp` aims to provide full compatability with IntelliJ [Http Client](https://www.jetbrains.com/help/idea/http-client-in-product-code-editor.html).

## Supported Features

| Feature                                                                                                           | Status | Commentary |
| ----------------------------------------------------------------------------------------------------------------- | ------ | ---------- |
| Environment Files                                                                                                 | ✅     |            |
| Global Variables                                                                                                  | ✅     |            |
| [Per-request Variables](https://www.jetbrains.com/help/idea/exploring-http-syntax.html#per_request_variables)     | ✅     |            |
| [In-place Variables](https://www.jetbrains.com/help/idea/exploring-http-syntax.html#in-place-variables)           | ✅     |            |
| [Dynamic Variables](https://www.jetbrains.com/help/idea/exploring-http-syntax.html#dynamic-variables)             | ✅     |            |
| [Iterate over variables](https://www.jetbrains.com/help/idea/exploring-http-syntax.html#collections-in-variables) | 🛑     |            |
| Response handlers, Response unit tests                                                                            | ✅     |            |
| Cookie jars                                                                                                       | 🛑     |            |
| gRPC requests                                                                                                     | 🛑     |            |
| WebSocket requests                                                                                                | 🛑     |            |
| GraphQL requests                                                                                                  | 🛑     |            |
| Postman Export                                                                                                    | 🚧     |            |

- ✅ Fully supported
- 🛑 Not yet supported
- 🚧 Work in progress

## Installation

### Binary releases

The easiest way for most users is simply to download the prebuilt binaries.
You can find binaries for various platforms on the
[release](https://github.com/alisa101rs/dothttp/releases) page.

### Cargo

First, install [cargo](https://rustup.rs/). Then:

```nu,no-run
> cargo install dothttp
```

You will need to use the stable release for this to work; if in doubt run

```nu,no-run
> rustup run stable cargo install dothttp
```

### Nix

You can also use `nix` (with flakes) to run and use `dothttp`:

```nu,no-run
> nix run github:alisa101rs/dothttp
```

## Usage

```nu
> dothttp --help
dothttp is a text-based scriptable HTTP client. It is a fork for dot-http. It is a simple language that resembles the actual HTTP protocol but with additional features to make it practical for someone who builds and tests APIs.

Usage: dothttp [OPTIONS] [FILES]...
       dothttp execute [OPTIONS] [FILES]...
       dothttp export-environment [OPTIONS]
       dothttp export-collection [OPTIONS] [FILES]...
       dothttp help [COMMAND]...

Arguments:
  [FILES]...
          List of request files to execute, optionally proceeded `:<number>` to execute only specified request out of all requests present in this file

          Example: request.http request-2.http request-3.http:2

Options:
      --request-format <REQUEST_FORMAT>
          The format of the request output. Only relevant if `--format=standard`.

          [possible values: %R - HTTP protocol, %N - request Name, %B - request Body, %H - request Headers]

          [default: "%N\n%R\n\n"]

      --response-format <RESPONSE_FORMAT>
          The format of the response output. Only relevant if `--format=standard`.

          [possible values: %R - HTTP protocol, %T - Response unit tests, %B - Response Body, %H - Response Headers]

          [default: "%R\n%H\n%B\n\n%T\n"]

      --accept-invalid-certs


      --format <FORMAT>
          Which mode to use to print result

          [default: standard]
          [possible values: standard, ci]

  -n, --environment-file <ENVIRONMENT_FILE>
          A file containing a JSON object that describes the initial values for variables

  -p, --snapshot <SNAPSHOT>
          A file containing a JSON object that persists variables between each invocation

  -e, --environment <ENVIRONMENT>
          The key value to use on the environment file

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

dothttp execute:
Execute requests
      --request-format <REQUEST_FORMAT>
          The format of the request output. Only relevant if `--format=standard`.

          [possible values: %R - HTTP protocol, %N - request Name, %B - request Body, %H - request Headers]

          [default: "%N\n%R\n\n"]

      --response-format <RESPONSE_FORMAT>
          The format of the response output. Only relevant if `--format=standard`.

          [possible values: %R - HTTP protocol, %T - Response unit tests, %B - Response Body, %H - Response Headers]

          [default: "%R\n%H\n%B\n\n%T\n"]

      --accept-invalid-certs


      --format <FORMAT>
          Which mode to use to print result

          [default: standard]
          [possible values: standard, ci]

  -n, --environment-file <ENVIRONMENT_FILE>
          A file containing a JSON object that describes the initial values for variables

  -p, --snapshot <SNAPSHOT>
          A file containing a JSON object that persists variables between each invocation

  -e, --environment <ENVIRONMENT>
          The key value to use on the environment file

  -h, --help
          Print help (see a summary with '-h')

  [FILES]...
          List of request files to execute, optionally proceeded `:<number>` to execute only specified request out of all requests present in this file

          Example: request.http request-2.http request-3.http:2

dothttp export-environment:
Export environment as postman_environment
  -n, --environment-file <ENVIRONMENT_FILE>
          A file containing a JSON object that describes the initial values for variables

  -p, --snapshot <SNAPSHOT>
          A file containing a JSON object that persists variables between each invocation

  -e, --environment <ENVIRONMENT>
          The key value to use on the environment file

      --name <NAME>
          Name for exported collection

          [default: dothttp-environment]

  -h, --help
          Print help

dothttp export-collection:
Export collection as postman_collection
      --name <NAME>
          Name for exported collection

          [default: dothttp-collection]

  -h, --help
          Print help (see a summary with '-h')

  [FILES]...
          List of request files to execute, optionally proceeded `:<number>` to execute only specified request out of all requests present in this file

          Example: request.http request-2.http request-3.http:2

dothttp help:
Print this message or the help of the given subcommand(s)
  [COMMAND]...
          Print help for the subcommand(s)
```

### Running requests

[Dothttp Request Format](docs/dothttp-format.md)

### Collection export to postman

[Exporting to postman](docs/postman-export.md)

## Contributing

Contributions and suggestions are very welcome!

Please create an issue before submitting a PR, PRs will only be accepted if they reference an existing issue.
If you have a suggested change please create an issue first so that we can discuss it.

## License

[Apache License 2.0](https://github.com/bayne/dothttp/blob/master/LICENSE)
