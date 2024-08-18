use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub(super) struct PostmanCollection {
    pub event: Vec<Event>,

    pub info: Information,

    /// Items are the basic unit for a Postman collection. You can think of them as corresponding
    /// to a single API endpoint. Each Item has one request and may have multiple API responses
    /// associated with it.
    pub item: Vec<Items>,

    pub variable: Vec<Variable>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) struct PostmanEnvironment {
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

/// Postman allows you to configure scripts to run when specific events occur. These scripts
/// are stored here, and can be referenced in the collection by their ID.
///
/// Defines a script associated with an associated event name
#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) struct Event {
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
pub(super) enum EventType {
    Test,
    Prerequest,
}

/// A script is a snippet of Javascript code that can be used to to perform setup or teardown
/// operations on a particular response.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) struct Script {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub exec: String,

    #[serde(rename = "type")]
    pub script_type: &'static str,
}

/// Collection variables allow you to define a set of variables, that are a *part of the
/// collection*, as opposed to environments, which are separate entities.
/// *Note: Collection variables must not contain any sensitive information.*
///
/// Using variables in your Postman requests eliminates the need to duplicate requests, which
/// can save a lot of time. Variables can be defined, and referenced to from any part of a
/// request.
#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub(super) struct Variable {
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
pub(super) struct Information {
    /// Every collection is identified by the unique value of this field. The value of this field
    /// is usually easiest to generate using a UID generator function. If you already have a
    /// collection, it is recommended that you maintain the same id since changing the id usually
    /// implies that is a different collection than it was originally.
    /// *Note: This field exists for compatibility reasons with Collection Format V1.*
    #[serde(rename = "_postman_id")]
    pub postman_id: String,

    /// A collection's friendly name is defined by this field. You would want to set this field
    /// to a value that would allow you to easily identify this collection among a bunch of other
    /// collections, as such outlining its usage or content.
    pub name: String,

    /// This should ideally hold a link to the Postman schema that is used to validate this
    /// collection. E.g: https://schema.getpostman.com/collection/v1
    pub schema: String,
}

impl Default for Information {
    fn default() -> Self {
        Self {
            schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                .to_owned(),
            name: "dothttp".to_owned(),
            postman_id: uuid::Uuid::new_v4().as_hyphenated().to_string(),
        }
    }
}

/// Items are entities which contain an actual HTTP request, and sample responses attached to
/// it.
///
/// One of the primary goals of Postman is to organize the development of APIs. To this end,
/// it is necessary to be able to group requests together. This can be achived using
/// 'Folders'. A folder just is an ordered set of requests.
#[derive(Clone, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct Items {
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

    pub request: Option<RequestClass>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variable: Vec<Variable>,

    /// Items are entities which contain an actual HTTP request, and sample responses attached to
    /// it. Folders may contain many items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item: Vec<Items>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct RequestClass {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<BodyClass>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<Header>,

    #[serde(default)]
    pub method: Method,

    pub url: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum Method {
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
pub(super) struct BodyClass {
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
pub(super) struct Options {
    pub raw: Raw,
}

/// This field contains the language in which the request body was written.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub(super) struct Raw {
    pub language: Language,
}

/// The language associated with the response.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[allow(unused)]
pub(super) enum Language {
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
pub(super) struct File {
    pub content: Option<String>,

    pub src: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(super) struct FormParameter {
    /// Override Content-Type header of this form data entity.
    pub content_type: Option<String>,

    /// When set to true, prevents this form data entity from being sent.
    pub disabled: Option<bool>,

    pub key: String,

    #[serde(rename = "type")]
    pub form_parameter_type: Option<String>,

    pub value: Option<String>,

    pub src: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub(super) struct UrlEncodedParameter {
    pub disabled: Option<bool>,

    pub key: String,

    pub value: Option<String>,
}

/// A representation for a list of headers
///
/// Represents a single HTTP Header
#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub(super) struct Header {
    /// If set to true, the current header will not be sent with requests.
    pub disabled: Option<bool>,

    /// This holds the LHS of the HTTP Header, e.g ``Content-Type`` or ``X-Custom-Header``
    pub key: String,

    /// The value (or the RHS) of the Header is stored in this field.
    pub value: String,
}

/// A variable may have multiple types. This field specifies the type of the variable.
#[derive(Clone, Debug, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
#[allow(unused)]
pub(super) enum VariableType {
    #[default]
    Default,
    Any,
    Boolean,
    Number,
    String,
}

/// Postman stores the type of data associated with this request in this field.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub(super) enum Mode {
    #[allow(unused)]
    #[serde(rename = "file")]
    File,

    #[serde(rename = "formdata")]
    Formdata,

    #[serde(rename = "raw")]
    Raw,

    #[serde(rename = "urlencoded")]
    Urlencoded,
}
