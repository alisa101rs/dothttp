#![allow(unused)]

use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub struct PostmanCollection {
    pub auth: Option<Auth>,

    pub event: Vec<Event>,

    pub info: Information,

    /// Items are the basic unit for a Postman collection. You can think of them as corresponding
    /// to a single API endpoint. Each Item has one request and may have multiple API responses
    /// associated with it.
    pub item: Vec<Items>,

    pub variable: Vec<Variable>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PostmanEnvironment {
    pub id: String,

    pub name: String,

    pub values: Vec<Variable>,
}

impl Default for PostmanEnvironment {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().as_hyphenated().to_string(),
            name: Default::default(),
            values: Default::default(),
        }
    }
}

/// Represents authentication helpers provided by Postman
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Auth {
    /// The attributes for [AWS
    /// Auth](http://docs.aws.amazon.com/AmazonS3/latest/dev/RESTAuthentication.html).
    pub awsv4: Option<Vec<AuthAttribute>>,

    /// The attributes for [Basic
    /// Authentication](https://en.wikipedia.org/wiki/Basic_access_authentication).
    pub basic: Option<Vec<AuthAttribute>>,

    /// The helper attributes for [Bearer Token
    /// Authentication](https://tools.ietf.org/html/rfc6750)
    pub bearer: Option<Vec<AuthAttribute>>,

    /// The attributes for [Digest
    /// Authentication](https://en.wikipedia.org/wiki/Digest_access_authentication).
    pub digest: Option<Vec<AuthAttribute>>,

    /// The attributes for [Hawk Authentication](https://github.com/hueniverse/hawk)
    pub hawk: Option<Vec<AuthAttribute>>,

    pub noauth: Option<serde_json::Value>,

    /// The attributes for [NTLM
    /// Authentication](https://msdn.microsoft.com/en-us/library/cc237488.aspx)
    pub ntlm: Option<Vec<AuthAttribute>>,

    /// The attributes for [OAuth2](https://oauth.net/1/)
    pub oauth1: Option<Vec<AuthAttribute>>,

    /// Helper attributes for [OAuth2](https://oauth.net/2/)
    pub oauth2: Option<Vec<AuthAttribute>>,

    #[serde(rename = "type")]
    pub auth_type: AuthType,
}

/// Represents an attribute for any authorization method provided by Postman. For example
/// `username` and `password` are set as auth attributes for Basic Authentication method.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AuthAttribute {
    pub key: String,

    #[serde(rename = "type")]
    pub auth_type: Option<String>,

    pub value: Option<serde_json::Value>,
}

/// Postman allows you to configure scripts to run when specific events occur. These scripts
/// are stored here, and can be referenced in the collection by their ID.
///
/// Defines a script associated with an associated event name
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Event {
    // /// Indicates whether the event is disabled. If absent, the event is assumed to be enabled.
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub disabled: Option<bool>,

    // /// A unique identifier for the enclosing event.
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub id: Option<String>,
    /// Can be set to `test` or `prerequest` for test scripts or pre-request scripts respectively.
    pub listen: EventType,

    pub script: Script,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Test,
    Prerequest,
}

/// A script is a snippet of Javascript code that can be used to to perform setup or teardown
/// operations on a particular response.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Script {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub exec: String,

    #[serde(rename = "type")]
    pub script_type: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub struct UrlClass {
    /// Contains the URL fragment (if any). Usually this is not transmitted over the network, but
    /// it could be useful to store this in some cases.
    pub hash: Option<String>,

    /// The host for the URL, E.g: api.yourdomain.com. Can be stored as a string or as an array
    /// of strings.
    pub host: Option<Host>,

    pub path: Option<UrlPath>,

    /// The port number present in this URL. An empty value implies 80/443 depending on whether
    /// the protocol field contains http/https.
    pub port: Option<String>,

    /// The protocol associated with the request, E.g: 'http'
    pub protocol: Option<String>,

    /// An array of QueryParams, which is basically the query string part of the URL, parsed into
    /// separate variables
    pub query: Option<Vec<QueryParam>>,

    /// The string representation of the request URL, including the protocol, host, path, hash,
    /// query parameter(s) and path variable(s).
    pub raw: Option<String>,

    /// Postman supports path variables with the syntax `/path/:variableName/to/somewhere`. These
    /// variables are stored in this field.
    pub variable: Option<Vec<Variable>>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PathClass {
    #[serde(rename = "type")]
    pub path_type: Option<String>,

    pub value: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct QueryParam {
    pub description: Option<DescriptionUnion>,

    /// If set to true, the current query parameter will not be sent with the request.
    pub disabled: Option<bool>,

    pub key: Option<String>,

    pub value: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Description {
    /// The content of the description goes here, as a raw string.
    pub content: Option<String>,

    /// Holds the mime type of the raw description content. E.g: 'text/markdown' or 'text/html'.
    /// The type is used to correctly render the description when generating documentation, or in
    /// the Postman app.
    #[serde(rename = "type")]
    pub description_type: Option<String>,

    /// Description can have versions associated with it, which should be put in this property.
    pub version: Option<serde_json::Value>,
}

/// Collection variables allow you to define a set of variables, that are a *part of the
/// collection*, as opposed to environments, which are separate entities.
/// *Note: Collection variables must not contain any sensitive information.*
///
/// Using variables in your Postman requests eliminates the need to duplicate requests, which
/// can save a lot of time. Variables can be defined, and referenced to from any part of a
/// request.
#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub struct Variable {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<DescriptionUnion>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,

    /// A variable ID is a unique user-defined value that identifies the variable within a
    /// collection. In traditional terms, this would be a variable name.

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// A variable key is a human friendly value that identifies the variable within a
    /// collection. In traditional terms, this would be a variable name.
    pub key: String,

    /// Variable name

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// When set to true, indicates that this variable has been set by Postman

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<bool>,

    /// A variable may have multiple types. This field specifies the type of the variable.
    #[serde(rename = "type")]
    pub variable_type: VariableType,

    /// The value that a variable holds in this collection. Ultimately, the variables will be
    /// replaced by this value, when say running a set of requests from a collection
    pub value: serde_json::Value,
}

/// Detailed description of the info block
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Information {
    /// Every collection is identified by the unique value of this field. The value of this field
    /// is usually easiest to generate using a UID generator function. If you already have a
    /// collection, it is recommended that you maintain the same id since changing the id usually
    /// implies that is a different collection than it was originally.
    /// *Note: This field exists for compatibility reasons with Collection Format V1.*
    #[serde(rename = "_postman_id")]
    pub postman_id: String,

    pub description: Option<DescriptionUnion>,

    /// A collection's friendly name is defined by this field. You would want to set this field
    /// to a value that would allow you to easily identify this collection among a bunch of other
    /// collections, as such outlining its usage or content.
    pub name: String,

    /// This should ideally hold a link to the Postman schema that is used to validate this
    /// collection. E.g: https://schema.getpostman.com/collection/v1
    pub schema: String,

    #[serde(rename = "version")]
    pub version: Option<CollectionVersion>,
}

impl Default for Information {
    fn default() -> Self {
        Self {
            schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                .to_owned(),
            name: "dothttp".to_owned(),
            postman_id: uuid::Uuid::new_v4().as_hyphenated().to_string(),
            version: None,
            description: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CollectionVersionClass {
    /// A human friendly identifier to make sense of the version numbers. E.g: 'beta-3'
    pub identifier: Option<String>,

    /// Increment this number if you make changes to the collection that changes its behaviour.
    /// E.g: Removing or adding new test scripts. (partly or completely).
    pub major: i64,

    pub meta: Option<serde_json::Value>,

    /// You should increment this number if you make changes that will not break anything that
    /// uses the collection. E.g: removing a folder.
    pub minor: i64,

    /// Ideally, minor changes to a collection should result in the increment of this number.
    pub patch: i64,
}

/// Items are entities which contain an actual HTTP request, and sample responses attached to
/// it.
///
/// One of the primary goals of Postman is to organize the development of APIs. To this end,
/// it is necessary to be able to group requests together. This can be achived using
/// 'Folders'. A folder just is an ordered set of requests.
#[derive(Clone, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Items {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<DescriptionUnion>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event: Vec<Event>,

    /// A unique ID that is used to identify collections internally
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// A human readable identifier for the current item.
    ///
    /// A folder's friendly name is defined by this field. You would want to set this field to a
    /// value that would allow you to easily identify this folder.
    pub name: String,

    /// Set of configurations used to alter the usual behavior of sending the request
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_profile_behavior: Option<ProtocolProfileBehavior>,

    pub request: Option<RequestClass>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Vec<Option<Response>>>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variable: Vec<Variable>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,

    /// Items are entities which contain an actual HTTP request, and sample responses attached to
    /// it. Folders may contain many items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item: Vec<Items>,
}

/// Set of configurations used to alter the usual behavior of sending the request
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolProfileBehavior {
    /// Disable body pruning for GET, COPY, HEAD, PURGE and UNLOCK request methods.
    pub disable_body_pruning: Option<bool>,

    /// Automatically follow redirects.
    pub follow_redirects: Option<bool>,

    /// Disable cookie jar.
    pub disable_cookies: Option<bool>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RequestClass {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<BodyClass>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate: Option<Certificate>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<DescriptionUnion>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<Header>,

    #[serde(default)]
    pub method: Method,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxyConfig>,

    pub url: Url,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Method {
    #[default]
    Get,
    Post,
    Patch,
    Put,
    Delete,
    Options,
}

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BodyClass {
    /// When set to true, prevents request body from being sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<File>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formdata: Option<Vec<FormParameter>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Options>,

    /// Postman stores the type of data associated with this request in this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urlencoded: Option<Vec<UrlEncodedParameter>>,
}

/// This field contains the request body options.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Options {
    pub raw: Raw,
}

/// This field contains the language in which the request body was written.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Raw {
    pub language: Language,
}

/// The language associated with the response.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub enum Language {
    #[serde(rename = "html")]
    Html,

    #[serde(rename = "json")]
    Json,

    #[serde(rename = "text")]
    Text,

    #[serde(rename = "xml")]
    Xml,

    #[serde(rename = "javascript")]
    Javascript,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct File {
    pub content: Option<String>,

    pub src: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FormParameter {
    /// Override Content-Type header of this form data entity.
    pub content_type: Option<String>,

    pub description: Option<DescriptionUnion>,

    /// When set to true, prevents this form data entity from being sent.
    pub disabled: Option<bool>,

    pub key: String,

    #[serde(rename = "type")]
    pub form_parameter_type: Option<String>,

    pub value: Option<String>,

    pub src: Option<FormParameterSrcUnion>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum FormParameterSrcUnion {
    File(String),

    Files(Vec<String>),
}

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub struct UrlEncodedParameter {
    pub description: Option<DescriptionUnion>,

    pub disabled: Option<bool>,

    pub key: String,

    pub value: Option<String>,
}

/// A representation of an ssl certificate
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Certificate {
    /// An object containing path to file certificate, on the file system
    pub cert: Option<Cert>,

    /// An object containing path to file containing private key, on the file system
    pub key: Option<Key>,

    /// A list of Url match pattern strings, to identify Urls this certificate can be used for.
    pub matches: Option<Vec<Option<serde_json::Value>>>,

    /// A name for the certificate for user reference
    pub name: Option<String>,

    /// The passphrase for the certificate
    pub passphrase: Option<String>,
}

/// An object containing path to file certificate, on the file system
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Cert {
    /// The path to file containing key for certificate, on the file system
    pub src: Option<serde_json::Value>,
}

/// An object containing path to file containing private key, on the file system
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Key {
    /// The path to file containing key for certificate, on the file system
    pub src: Option<serde_json::Value>,
}

/// A representation for a list of headers
///
/// Represents a single HTTP Header
#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub struct Header {
    pub description: Option<DescriptionUnion>,

    /// If set to true, the current header will not be sent with requests.
    pub disabled: Option<bool>,

    /// This holds the LHS of the HTTP Header, e.g ``Content-Type`` or ``X-Custom-Header``
    pub key: String,

    /// The value (or the RHS) of the Header is stored in this field.
    pub value: String,
}

/// Using the Proxy, you can configure your custom proxy into the postman for particular url
/// match
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProxyConfig {
    /// When set to true, ignores this proxy configuration entity
    #[serde(rename = "disabled")]
    pub disabled: Option<bool>,

    /// The proxy server host
    #[serde(rename = "host")]
    pub host: Option<String>,

    /// The Url match for which the proxy config is defined
    #[serde(rename = "match")]
    pub proxy_config_match: Option<String>,

    /// The proxy server port
    #[serde(rename = "port")]
    pub port: Option<i64>,

    /// The tunneling details for the proxy config
    #[serde(rename = "tunnel")]
    pub tunnel: Option<bool>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ResponseClass {
    /// The raw text of the response.
    #[serde(rename = "body")]
    pub body: Option<String>,

    /// The numerical response code, example: 200, 201, 404, etc.
    #[serde(rename = "code")]
    pub code: Option<i64>,

    #[serde(rename = "cookie")]
    pub cookie: Option<Vec<Cookie>>,

    #[serde(rename = "header")]
    pub header: Option<Headers>,

    /// A unique, user defined identifier that can  be used to refer to this response from
    /// requests.
    #[serde(rename = "id")]
    pub id: Option<String>,

    #[serde(rename = "originalRequest")]
    pub original_request: Option<RequestUnion>,

    /// The time taken by the request to complete. If a number, the unit is milliseconds. If the
    /// response is manually created, this can be set to `null`.
    #[serde(rename = "responseTime")]
    pub response_time: Option<ResponseTime>,

    /// The response status, e.g: '200 OK'
    #[serde(rename = "status")]
    pub status: Option<String>,
}

/// A Cookie, that follows the [Google Chrome
/// format](https://developer.chrome.com/extensions/cookies)
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Cookie {
    /// The domain for which this cookie is valid.
    #[serde(rename = "domain")]
    pub domain: String,

    /// When the cookie expires.
    #[serde(rename = "expires")]
    pub expires: Option<String>,

    /// Custom attributes for a cookie go here, such as the [Priority
    /// Field](https://code.google.com/p/chromium/issues/detail?id=232693)
    #[serde(rename = "extensions")]
    pub extensions: Option<Vec<Option<serde_json::Value>>>,

    /// True if the cookie is a host-only cookie. (i.e. a request's URL domain must exactly match
    /// the domain of the cookie).
    #[serde(rename = "hostOnly")]
    pub host_only: Option<bool>,

    /// Indicates if this cookie is HTTP Only. (if True, the cookie is inaccessible to
    /// client-side scripts)
    #[serde(rename = "httpOnly")]
    pub http_only: Option<bool>,

    #[serde(rename = "maxAge")]
    pub max_age: Option<String>,

    /// This is the name of the Cookie.
    #[serde(rename = "name")]
    pub name: Option<String>,

    /// The path associated with the Cookie.
    #[serde(rename = "path")]
    pub path: String,

    /// Indicates if the 'secure' flag is set on the Cookie, meaning that it is transmitted over
    /// secure connections only. (typically HTTPS)
    #[serde(rename = "secure")]
    pub secure: Option<bool>,

    /// True if the cookie is a session cookie.
    #[serde(rename = "session")]
    pub session: Option<bool>,

    /// The value of the Cookie.
    #[serde(rename = "value")]
    pub value: Option<String>,
}

/// The host for the URL, E.g: api.yourdomain.com. Can be stored as a string or as an array
/// of strings.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Host {
    String(String),

    StringArray(Vec<String>),
}

/// If object, contains the complete broken-down URL for this request. If string, contains
/// the literal request URL.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Url {
    String(String),
    UrlClass(UrlClass),
}

impl Default for Url {
    fn default() -> Self {
        Self::UrlClass(Default::default())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum UrlPath {
    String(String),

    UnionArray(Vec<PathElement>),
}

/// The complete path of the current url, broken down into segments. A segment could be a
/// string, or a path variable.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum PathElement {
    PathClass(PathClass),

    String(String),
}

/// A Description can be a raw text, or be an object, which holds the description along with
/// its format.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum DescriptionUnion {
    Description(Description),

    String(String),
}

/// Postman allows you to version your collections as they grow, and this field holds the
/// version number. While optional, it is recommended that you use this field to its fullest
/// extent!
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum CollectionVersion {
    CollectionVersionClass(CollectionVersionClass),

    String(String),
}

/// A request represents an HTTP request. If a string, the string is assumed to be the
/// request URL and the method is assumed to be 'GET'.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum RequestUnion {
    RequestClass(RequestClass),

    String(String),
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum HeaderUnion {
    HeaderArray(Vec<Header>),

    String(String),
}

/// A response represents an HTTP response.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Response {
    AnythingArray(Vec<Option<serde_json::Value>>),

    Bool(bool),

    Double(f64),

    Integer(i64),

    ResponseClass(ResponseClass),

    String(String),
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Headers {
    String(String),

    UnionArray(Vec<HeaderElement>),
}

/// No HTTP request is complete without its headers, and the same is true for a Postman
/// request. This field is an array containing all the headers.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum HeaderElement {
    Header(Header),

    String(String),
}

/// The time taken by the request to complete. If a number, the unit is milliseconds. If the
/// response is manually created, this can be set to `null`.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum ResponseTime {
    Double(f64),

    String(String),
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub enum AuthType {
    #[serde(rename = "awsv4")]
    Awsv4,

    #[serde(rename = "basic")]
    Basic,

    #[serde(rename = "bearer")]
    Bearer,

    #[serde(rename = "digest")]
    Digest,

    #[serde(rename = "hawk")]
    Hawk,

    #[serde(rename = "noauth")]
    Noauth,

    #[serde(rename = "ntlm")]
    Ntlm,

    #[serde(rename = "oauth1")]
    Oauth1,

    #[serde(rename = "oauth2")]
    Oauth2,
}

/// Returns `Noauth` for AuthType by default
impl Default for AuthType {
    fn default() -> AuthType {
        AuthType::Noauth
    }
}

/// A variable may have multiple types. This field specifies the type of the variable.
#[derive(Clone, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    #[default]
    Default,
    Any,
    Boolean,
    Number,
    String,
}

/// Postman stores the type of data associated with this request in this field.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub enum Mode {
    #[serde(rename = "file")]
    File,

    #[serde(rename = "formdata")]
    Formdata,

    #[serde(rename = "raw")]
    Raw,

    #[serde(rename = "urlencoded")]
    Urlencoded,
}
