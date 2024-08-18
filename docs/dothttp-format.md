# The request

The request format is intended to resemble HTTP as close as possible.
HTTP was initially designed to be human-readable and simple, so why not use that?

**simple.http**

```nu
> cat requests/simple.http
GET https://httpbin.org/get
Accept: */*
```

Executing that script just prints the response to stdout:
```nu
> dothttp requests/simple.http
[requests/simple.http / #1]
GET https://httpbin.org/get

HTTP/2 200 OK
date: Sun, 18 Aug 2024 10:34:30 GMT
content-type: application/json
content-length: 221
server: gunicorn/19.9.0
access-control-allow-origin: *
access-control-allow-credentials: true

{
  "args": {},
  "headers": {
    "Accept": "*/*",
    "Host": "httpbin.org",
    "X-Amzn-Trace-Id": "Root=1-66c1ce36-150553d77f39544c40b6bc1e"
  },
  "origin": "138.64.99.135",
  "url": "https://httpbin.org/get"
}
```

## Variables

Use variables to build the scripts dynamically, either pulling data from your environment file or from a previous request's response handler.

**simple_with_variables.http**
```nu
> cat requests/simple_with_variables.http
POST https://httpbin.org/post
Accept: */*
X-Auth-Token: {{token}}

{
    "id": {{env_id}}
}
```

**http-client.env.json**
```nu
> cat requests/http-client.env.json
{
  "dev": {
    "env_id": "42",
    "token": "MyDevToken"
  }
}
```

Executing this requests with `dev` environment will result in `{{variables}}` being replaces with their actual values:
```nu
> dothttp -n requests/http-client.env.json -e dev requests/simple_with_variables.http
[requests/simple_with_variables.http / #1]
POST https://httpbin.org/post

HTTP/2 200 OK
date: Sun, 18 Aug 2024 10:34:31 GMT
content-type: application/json
content-length: 383
server: gunicorn/19.9.0
access-control-allow-origin: *
access-control-allow-credentials: true

{
  "args": {},
  "data": "{\n    \"id\": 42\n}",
  "files": {},
  "form": {},
  "headers": {
    "Accept": "*/*",
    "Content-Length": "16",
    "Host": "httpbin.org",
    "X-Amzn-Trace-Id": "Root=1-66c1ce37-50763ad01af607555dc0da6b",
    "X-Auth-Token": "MyDevToken"
  },
  "json": {
    "id": 42
  },
  "origin": "138.64.99.135",
  "url": "https://httpbin.org/post"
}
```

## Environment file

Use an environment file to control what initial values variables have

**multi.env.json**
```nu
> cat requests/multi.env.json
{
  "dev": {
    "env_id": "42",
    "token": "MyDevToken"
  },
  "prod": {
    "env_id": "24",
    "token": "MyProdToken"
  }
}
```

Now we can execute the same request file with different environments:
```nu
> dothttp -n requests/multi.env.json -e dev requests/simple_with_variables.http
[requests/simple_with_variables.http / #1]
POST https://httpbin.org/post

HTTP/2 200 OK
date: Sun, 18 Aug 2024 10:34:32 GMT
content-type: application/json
content-length: 383
server: gunicorn/19.9.0
access-control-allow-origin: *
access-control-allow-credentials: true

{
  "args": {},
  "data": "{\n    \"id\": 42\n}",
  "files": {},
  "form": {},
  "headers": {
    "Accept": "*/*",
    "Content-Length": "16",
    "Host": "httpbin.org",
    "X-Amzn-Trace-Id": "Root=1-66c1ce38-6099ef415ac8cee725b928b3",
    "X-Auth-Token": "MyDevToken"
  },
  "json": {
    "id": 42
  },
  "origin": "138.64.99.135",
  "url": "https://httpbin.org/post"
}

> dothttp -n requests/multi.env.json -e prod requests/simple_with_variables.http
[requests/simple_with_variables.http / #1]
POST https://httpbin.org/post

HTTP/2 200 OK
date: Sun, 18 Aug 2024 10:34:33 GMT
content-type: application/json
content-length: 384
server: gunicorn/19.9.0
access-control-allow-origin: *
access-control-allow-credentials: true

{
  "args": {},
  "data": "{\n    \"id\": 24\n}",
  "files": {},
  "form": {},
  "headers": {
    "Accept": "*/*",
    "Content-Length": "16",
    "Host": "httpbin.org",
    "X-Amzn-Trace-Id": "Root=1-66c1ce39-3ac5624d296c34242cc652f9",
    "X-Auth-Token": "MyProdToken"
  },
  "json": {
    "id": 24
  },
  "origin": "138.64.99.135",
  "url": "https://httpbin.org/post"
}
```

## Response Handlers

Use previous requests to populate some of the data in future requests

**response_handler.http**

```nu
> cat requests/response_handler.http
### Get Data

POST http://httpbin.org/post
Content-Type: application/json

{
    "token": "sometoken",
    "id": "237"
}

> {%
   client.global.set("auth_token", response.body["json"]["token"]);
   client.global.set("some_id", response.body["json"]["id"]);
%}

### Make request with data

PUT http://httpbin.org/put
X-Auth-Token: {{auth_token}}
Content-Type: application/json

{
    "id": "{{some_id}}"
}

> {%
    client.test("correct token is present", () => {
        client.assert(response.body["headers"]["X-Auth-Token"] == client.global.get("auth_token"));
    });
%}
```

Execution result of this .http file:

```nu
> dothttp requests/response_handler.http
[requests/response_handler.http / Get Data]
POST http://httpbin.org/post

HTTP/1.1 200 OK
date: Sun, 18 Aug 2024 10:34:33 GMT
content-type: application/json
content-length: 454
connection: keep-alive
server: gunicorn/19.9.0
access-control-allow-origin: *
access-control-allow-credentials: true

{
  "args": {},
  "data": "{\n    \"token\": \"sometoken\",\n    \"id\": \"237\"\n}",
  "files": {},
  "form": {},
  "headers": {
    "Accept": "*/*",
    "Content-Length": "45",
    "Content-Type": "application/json",
    "Host": "httpbin.org",
    "X-Amzn-Trace-Id": "Root=1-66c1ce39-5c638c296e6ccc05088258d5"
  },
  "json": {
    "id": "237",
    "token": "sometoken"
  },
  "origin": "138.64.99.135",
  "url": "http://httpbin.org/post"
}

[requests/response_handler.http / Make request with data]
PUT http://httpbin.org/put

HTTP/1.1 200 OK
date: Sun, 18 Aug 2024 10:34:33 GMT
content-type: application/json
content-length: 429
connection: keep-alive
server: gunicorn/19.9.0
access-control-allow-origin: *
access-control-allow-credentials: true

{
  "args": {},
  "data": "{\n    \"id\": \"237\"\n}",
  "files": {},
  "form": {},
  "headers": {
    "Accept": "*/*",
    "Content-Length": "19",
    "Content-Type": "application/json",
    "Host": "httpbin.org",
    "X-Amzn-Trace-Id": "Root=1-66c1ce39-18e32b156f305910059c4a48",
    "X-Auth-Token": "sometoken"
  },
  "json": {
    "id": "237"
  },
  "origin": "138.64.99.135",
  "url": "http://httpbin.org/put"
}

Test `correct token is present`: OK
```

For the rest of the feature, please refer to [ijhttp documentation](https://www.jetbrains.com/help/idea/exploring-http-syntax.html).
