## **This is a fork of existing tool [dot-http](https://github.com/bayne/dot-http)**

# dothttp

[![codecov](https://codecov.io/gh/alisa101rs/dothttp/graph/badge.svg?token=9GWCS5H23D)](https://codecov.io/gh/alisa101rs/dothttp)
[![Crates.io](https://img.shields.io/crates/v/dothttp.svg)](https://crates.io/crates/dothttp)

dothttp is a text-based scriptable HTTP client. 
It is a simple language that resembles the actual HTTP protocol but with just a smidgen of magic to make it more practical for someone who builds and tests APIs.

dothttp aims to provide full compatability with IntelliJ [Http Client](https://www.jetbrains.com/help/idea/http-client-in-product-code-editor.html).

Current list of feature support:
- [x] Environment Files
- [x] Variables
- [x] Special variables: `$random`, `$timestamp` and `$isoTimestamp`
- [x] Response handlers
- [x] Response tests
- [ ] Posting request bodies from files
- [ ] Pre-request scripts
- [ ] Cookies
- [ ] gRPC requests
- [ ] WebSocket requests
- [ ] GraphQL
 
## Things to consider
- [ ] terminal UI with [ratatui](https://github.com/ratatui-org/ratatui)
- [ ] stress test support

## Installation

### Binary releases

The easiest way for most users is simply to download the prebuilt binaries.
You can find binaries for various platforms on the
[release](https://github.com/alisa101rs/dothttp/releases) page.

### Cargo

First, install [cargo](https://rustup.rs/). Then:

```bash,no_run
$ cargo install dothttp
```

You will need to use the stable release for this to work; if in doubt run

```bash,no_run
rustup run stable cargo install dothttp
```

## Usage

See `dothttp --help` for usage.

### The request

The request format is intended to resemble HTTP as close as possible. HTTP was initially designed to be human-readable and simple, so why not use that?

**simple.http**
```text,no_run
GET http://httpbin.org
Accept: */*
```
Executing that script just prints the response to stdout:
```text,no_run
$ dothttp simple.http
GET http://httpbin.org/get

HTTP/1.1 200 OK
access-control-allow-credentials: true
access-control-allow-origin: *
content-type: application/json
date: Sat, 18 Jan 2020 20:48:50 GMT
referrer-policy: no-referrer-when-downgrade
server: nginx
x-content-type-options: nosniff
x-frame-options: DENY
x-xss-protection: 1; mode=block
content-length: 170
connection: keep-alive

{
  "args": {},
  "headers": {
    "Accept": "*/*",
    "Host": "httpbin.org"
  },
  "url": "https://httpbin.org/get"
}
```

### Variables

Use variables to build the scripts dynamically, either pulling data from your environment file or from a previous request's response handler.

**simple_with_variables.http**
```text,no_run
POST http://httpbin.org/post
Accept: */*
X-Auth-Token: {{token}}

{
    "id": {{env_id}}
}
```

**http-client.env.json**
```text,no_run
{
    "dev": {
        "env_id": 42,
        "token": "SuperSecretToken"
    }
}
```

Note that the variables are replaced by their values
```text,no_run
$ dothttp simple_with_variables.http
POST http://httpbin.org/post

HTTP/1.1 200 OK
access-control-allow-credentials: true
access-control-allow-origin: *
content-type: application/json
date: Sat, 18 Jan 2020 20:55:24 GMT
referrer-policy: no-referrer-when-downgrade
server: nginx
x-content-type-options: nosniff
x-frame-options: DENY
x-xss-protection: 1; mode=block
content-length: 342
connection: keep-alive

{
  "args": {},
  "data": "{\r\n    \"id\": 42\r\n}",
  "files": {},
  "form": {},
  "headers": {
    "Accept": "*/*",
    "Content-Length": "18",
    "Host": "httpbin.org",
    "X-Auth-Token": "SuperSecretToken"
  },
  "json": {
    "id": 42
  },
  "url": "https://httpbin.org/post"
}
```

### Environment file

Use an environment file to control what initial values variables have

**http-client.env.json**
```text,no_run
{
    "dev": {
        "host": localhost,
        "token": "SuperSecretToken"
    },
    "prod": {
        "host": example.com,
        "token": "ProductionToken"
    }
}
```

**env_demo.http**
```text,no_run
GET http://{{host}}
X-Auth-Token: {{token}}
```

Specifying different environments when invoking the command results in different values
for the variables in the script

```text,no_run
$ dothttp -e dev env_demo.http
GET http://localhost
X-Auth-Token: SuperSecretToken

$ dothttp -e prod env_demo.htp
GET http://example.com
X-Auth-Token: ProductionToken
```

### Response handler

Use previous requests to populate some of the data in future requests

**response_handler.http**
```text,no_run
POST http://httpbin.org/post
Content-Type: application/json

{
    "token": "sometoken",
    "id": 237
}

> {%
   client.global.set('auth_token', response.body.json.token);
   client.global.set('some_id', response.body.json.id);
%}

###

PUT http://httpbin.org/put
X-Auth-Token: {{auth_token}}

{
    "id": {{some_id}}
}
```

Data from a previous request

```text,no_run
$ dothttp test.http
POST http://httpbin.org/post

HTTP/1.1 200 OK
access-control-allow-credentials: true
access-control-allow-origin: *
content-type: application/json
date: Sat, 18 Jan 2020 21:01:59 GMT
referrer-policy: no-referrer-when-downgrade
server: nginx
x-content-type-options: nosniff
x-frame-options: DENY
x-xss-protection: 1; mode=block
content-length: 404
connection: keep-alive

{
  "args": {},
  "data": "{\r\n    \"token\": \"sometoken\",\r\n    \"id\": 237\r\n}",
  "files": {},
  "form": {},
  "headers": {
    "Accept": "*/*",
    "Content-Length": "46",
    "Content-Type": "application/json",
    "Host": "httpbin.org"
  },
  "json": {
    "id": 237,
    "token": "sometoken"
  },
  "url": "https://httpbin.org/post"
}
```

## Contributing

Contributions and suggestions are very welcome!

Please create an issue before submitting a PR, PRs will only be accepted if they reference an existing issue. 
If you have a suggested change please create an issue first so that we can discuss it.

## License
[Apache License 2.0](https://github.com/bayne/dothttp/blob/master/LICENSE)

