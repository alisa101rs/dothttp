use std::fmt::Write;

use color_eyre::eyre::{Result, WrapErr};
use postman::BodyClass;

use serde_json::Value as DynValue;

use crate::{
    parser::{self},
    source::SourceItem,
    EnvironmentProvider,
};

mod postman;

pub fn environment(name: String, env: impl EnvironmentProvider) -> Result<()> {
    let mut environment = postman::PostmanEnvironment {
        name,
        ..Default::default()
    };

    let DynValue::Object(snapshot) = env.snapshot() else {
        panic!("Snapshot should be valid object")
    };

    for (name, value) in snapshot {
        let var = postman::Variable {
            key: name,
            value,
            variable_type: postman::VariableType::Default,
            ..Default::default()
        };

        environment.values.push(var);
    }

    let mut writer = std::io::stdout();
    serde_json::to_writer_pretty(&mut writer, &environment)
        .wrap_err("Failed to write to output")?;

    Ok(())
}

pub fn collection<'a, I>(name: String, sources: I) -> Result<()>
where
    I: Iterator<Item = SourceItem<'a>> + 'a,
{
    let mut exporter = CollectionExporter::new(name);

    for source in sources {
        exporter.add_request(source)?;
    }

    exporter.export()?;

    Ok(())
}

#[derive(Debug, Default)]
struct CollectionExporter {
    inner: postman::PostmanCollection,
}

impl CollectionExporter {
    fn new(name: String) -> Self {
        Self {
            inner: postman::PostmanCollection {
                info: postman::Information {
                    name,
                    ..Default::default()
                },
                event: vec![
                    PreRequestScriptHelper::collection_script(),
                    ResponseHandlerHelper::collection_script(),
                ],
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn add_request<'a>(&mut self, source: SourceItem<'a>) -> Result<()> {
        let parser::RequestScript {
            name,
            request,
            request_variables,
            pre_request_handler,
            handler,
            ..
        } = source.script;

        let request_name = name.clone().unwrap_or_else(|| source.name.to_owned());

        let mut item = postman::Items {
            name: request_name,
            ..Default::default()
        };
        item.event.extend(
            ResponseHandlerHelper::default()
                .handler(handler)
                .to_script(),
        );

        let parser::Request {
            method,
            target,
            headers,
            body,
            ..
        } = request;

        let mut pre_request_helper = PreRequestScriptHelper::default();

        let request = postman::RequestClass {
            method: method.into(),
            url: postman::Url::String(pre_request_helper.process(target)),
            body: Self::make_body(body, headers, &mut pre_request_helper),
            header: Self::make_headers(headers, &mut pre_request_helper),
            ..Default::default()
        };

        item.request = Some(request);

        item.event.extend(
            pre_request_helper
                .variables(request_variables.iter())
                .pre_request_handler(pre_request_handler)
                .to_script(),
        );

        Self::file(&mut self.inner.item, &source.name)
            .item
            .push(item);

        Ok(())
    }

    fn export(self) -> Result<()> {
        let mut writer = std::io::stdout();
        serde_json::to_writer_pretty(&mut writer, &self.inner)
            .wrap_err("Failed to write to output")?;

        Ok(())
    }

    fn file<'a>(items: &'a mut Vec<postman::Items>, name: &str) -> &'a mut postman::Items {
        if let Some(pos) = items.iter().position(|it| it.name == name) {
            &mut items[pos]
        } else {
            let item = postman::Items {
                name: name.to_owned(),
                ..Default::default()
            };
            items.push(item);
            items.last_mut().unwrap()
        }
    }

    fn make_headers(
        headers: &[parser::Header],
        helper: &mut PreRequestScriptHelper,
    ) -> Vec<postman::Header> {
        headers
            .iter()
            .map(|it| postman::Header {
                key: it.field_name.clone(),
                value: helper.process(&it.field_value),
                ..Default::default()
            })
            .collect()
    }

    fn make_body(
        body: &Option<parser::Value>,
        headers: &[parser::Header],
        helper: &mut PreRequestScriptHelper,
    ) -> Option<BodyClass> {
        let body = body.as_ref()?;
        let (mode, language) = Self::body_mode(headers);

        let processed_body = helper.process(body);
        let mut class = postman::BodyClass {
            mode: Some(mode),
            raw: Some(processed_body.clone()),
            options: language.map(|language| postman::Options {
                raw: postman::Raw { language },
            }),
            ..Default::default()
        };

        if mode == postman::Mode::Raw {
            return Some(class);
        }

        match mode {
            postman::Mode::Urlencoded => {
                let mut parameters = vec![];

                for (key, value) in
                    form_urlencoded::parse(class.raw.as_ref().unwrap().trim().as_bytes())
                {
                    parameters.push(postman::UrlEncodedParameter {
                        key: key.into_owned(),
                        value: Some(value.into_owned()),
                        ..Default::default()
                    });
                }
                class.urlencoded = Some(parameters);
            }
            postman::Mode::File | postman::Mode::Formdata => { /* unimplemented!() */ }
            postman::Mode::Raw => unreachable!(),
        }

        Some(class)
    }

    fn body_mode(headers: &[parser::Header]) -> (postman::Mode, Option<postman::Language>) {
        let Some(content_header) = headers
            .iter()
            .find(|h| h.field_name.eq_ignore_ascii_case("content-type"))
        else {
            return (postman::Mode::Raw, None);
        };

        let parser::Unprocessed::WithoutInline(ref content_value, _) =
            content_header.field_value.state
        else {
            return (postman::Mode::Raw, None);
        };

        match content_value.as_str() {
            x if x.starts_with("application/json") => {
                (postman::Mode::Raw, Some(postman::Language::Json))
            }
            "application/x-www-form-urlencoded" => (postman::Mode::Urlencoded, None),
            "multipart/form-data" => (postman::Mode::Formdata, None),
            _ => (postman::Mode::Raw, None),
        }
    }
}

#[derive(Debug, Default)]
struct PreRequestScriptHelper {
    body: String,
}

impl PreRequestScriptHelper {
    fn collection_script() -> postman::Event {
        const SRC: &'static str = include_str!("./collection_pre_script.js");

        postman::Event {
            listen: postman::EventType::Prerequest,
            script: postman::Script {
                exec: SRC.to_owned(),
                script_type: "text/javascript",
            },
        }
    }

    const PRELUDE: &'static str = "const { client, request } = shared.load(pm);\n";

    fn process(&mut self, v: &parser::Value) -> String {
        let (value, inline_scripts) = match v.state {
            parser::Unprocessed::WithInline {
                ref value,
                ref inline_scripts,
                ..
            } => (value, inline_scripts),
            parser::Unprocessed::WithoutInline(ref value, _) => {
                return value.clone();
            }
        };

        let mut value = value.clone();

        if inline_scripts.iter().all(|it| !it.script.starts_with("$")) {
            return value.clone();
        }

        for script in inline_scripts {
            if !script.script.starts_with("$") {
                continue;
            }

            match script.script.as_str() {
                "$timestamp" | "$isoTimestamp" | "$randomInt" | "$random.float" => {}
                "$uuid" => {
                    value = value.replace(&script.placeholder, "{{$guid}}");
                }
                "$random.uuid" => {
                    value = value.replace(&script.placeholder, "{{$randomUUID}}");
                }
                "$random.email" => {
                    value = value.replace(&script.placeholder, "{{$randomEmail}}");
                }
                "$random.integer" => {
                    value = value.replace(&script.placeholder, "{{$randomInt}}");
                }
                s if s.starts_with("$random.integer") => {
                    let fcall = s.strip_prefix("$random.integer").expect("start with it");
                    if !fcall.starts_with("(") || !fcall.ends_with(")") {
                        // This will be error in postman, should resolve manually
                        continue;
                    }

                    let args = fcall.strip_prefix("(").unwrap().strip_suffix(")").unwrap();
                    if args.is_empty() {
                        value = value.replace(&script.placeholder, "{{$randomInt}}");
                    }

                    if args.contains(",") {
                        let (min, max) = args.split_once(",").unwrap();
                        let Ok(min): Result<i64, _> = min.trim().parse() else {
                            continue;
                        };
                        let Ok(max): Result<i64, _> = max.trim().parse() else {
                            continue;
                        };
                        writeln!(
                            self.body,
                            "pm.variables.set('{}', Object.create({{toJSON: () => ({min} + ~~(Math.random() * ( ({max} - {min}) + 1 ))).toString() }}));",
                            s,
                        ).unwrap();
                    } else {
                        let Ok(max): Result<i64, _> = args.trim().parse() else {
                            continue;
                        };
                        writeln!(
                            self.body,
                            "pm.variables.set('{}', Object.create({{toJSON: () => (~~(Math.random() * ( {max} + 1 ))).toString() }}));",
                            s,
                        ).unwrap();
                    }
                }
                s if s.starts_with("$random.float") => {
                    let fcall = s.strip_prefix("$random.float").expect("start with it");
                    if !fcall.starts_with("(") || !fcall.ends_with(")") {
                        // This will be error in postman, should resolve manually
                        continue;
                    }

                    let args = fcall.strip_prefix("(").unwrap().strip_suffix(")").unwrap();
                    if args.is_empty() {
                        value = value.replace(&script.placeholder, "{{$random.float}}");
                    }

                    if args.contains(",") {
                        let (min, max) = args.split_once(",").unwrap();
                        let Ok(min): Result<f64, _> = min.trim().parse() else {
                            continue;
                        };
                        let Ok(max): Result<f64, _> = max.trim().parse() else {
                            continue;
                        };
                        writeln!(
                            self.body,
                            "pm.variables.set('{}', Object.create({{toJSON: () => ({min} + (Math.random() * ({max} - {min}))).toString() }}));",
                            s,
                        ).unwrap();
                    } else {
                        let Ok(max): Result<f64, _> = args.trim().parse() else {
                            continue;
                        };
                        writeln!(
                            self.body,
                            "pm.variables.set('{}', Object.create({{toJSON: () => (Math.random() * ( {max} + 1 )).toString() }}));",
                            s,
                        ).unwrap();
                    }
                }
                s if s.starts_with("$random.alphabetic") => {
                    let fcall = s.strip_prefix("$random.alphabetic").expect("start with it");
                    if !fcall.starts_with("(") || !fcall.ends_with(")") {
                        // This will be error in postman, should resolve manually
                        continue;
                    }
                    let arg = fcall.strip_prefix("(").unwrap().strip_suffix(")").unwrap();
                    let Ok(length): Result<usize, _> = arg.trim().parse() else {
                        continue;
                    };
                    writeln!(
                        self.body,
                        "pm.variables.set('{}', Object.create({{toJSON: () => shared.randomString({length}, 'a')}}));",
                        s,
                    )
                    .unwrap();
                }
                s if s.starts_with("$random.alphanumeric") => {
                    let fcall = s
                        .strip_prefix("$random.alphanumeric")
                        .expect("start with it");
                    if !fcall.starts_with("(") || !fcall.ends_with(")") {
                        // This will be error in postman, should resolve manually
                        continue;
                    }
                    let arg = fcall.strip_prefix("(").unwrap().strip_suffix(")").unwrap();
                    let Ok(length): Result<usize, _> = arg.trim().parse() else {
                        continue;
                    };
                    writeln!(
                        self.body,
                        "pm.variables.set('{}', Object.create({{toJSON: () => shared.randomString({length})}}));",
                        s,
                    )
                    .unwrap();
                }
                s if s.starts_with("$random.hexadecimal") => {
                    let fcall = s
                        .strip_prefix("$random.hexadecimal")
                        .expect("start with it");
                    if !fcall.starts_with("(") || !fcall.ends_with(")") {
                        // This will be error in postman, should resolve manually
                        continue;
                    }
                    let arg = fcall.strip_prefix("(").unwrap().strip_suffix(")").unwrap();
                    let Ok(length): Result<usize, _> = arg.trim().parse() else {
                        continue;
                    };
                    writeln!(
                        self.body,
                        "pm.variables.set('{}', Object.create({{toJSON: () => shared.randomString({length}, 'h')}}));",
                        s,
                    )
                    .unwrap();
                }
                _ => {}
            }
        }

        value
    }

    fn to_script(&mut self) -> Option<postman::Event> {
        if self.body.is_empty() {
            return None;
        }

        let event = postman::Event {
            listen: postman::EventType::Prerequest,
            script: postman::Script {
                exec: std::mem::replace(&mut self.body, Default::default()),
                script_type: "text/javascript",
            },
        };

        Some(event)
    }

    fn variables<'a, K>(&mut self, v: impl Iterator<Item = &'a (K, parser::Value)>) -> &mut Self
    where
        K: AsRef<str> + 'a,
    {
        use std::fmt::Write;

        for (k, value) in v {
            let name = k.as_ref();
            let value = self.process(value);
            writeln!(
                &mut self.body,
                "pm.variables.set(\"{name}\", pm.variables.replaceIn(\"{value}\"));"
            )
            .unwrap();
        }

        self
    }

    fn add_prelude(&mut self) {
        self.body.push_str(Self::PRELUDE);
    }

    fn pre_request_handler(&mut self, handler: &Option<parser::Handler>) -> &mut Self {
        let Some(handler) = handler else { return self };

        self.add_prelude();
        self.body.push_str(&handler.script);

        self
    }
}

#[derive(Debug, Default)]
struct ResponseHandlerHelper {
    body: String,
}

impl ResponseHandlerHelper {
    fn collection_script() -> postman::Event {
        const SRC: &'static str = include_str!("./collection_post_script.js");

        postman::Event {
            listen: postman::EventType::Test,
            script: postman::Script {
                exec: SRC.to_owned(),
                script_type: "text/javascript",
            },
        }
    }

    const PRELUDE: &'static str = "const { response, client } = shared.load(pm);\n";

    fn add_prelude(&mut self) {
        assert!(self.body.is_empty());
        self.body = Self::PRELUDE.to_owned();
    }

    fn handler(&mut self, handler: &Option<parser::Handler>) -> &mut Self {
        let Some(handler) = handler else { return self };
        if self.body.is_empty() {
            self.add_prelude();
        }
        self.body.push_str(&handler.script);

        self
    }

    fn to_script(&mut self) -> Option<postman::Event> {
        if self.body.is_empty() {
            return None;
        }

        let event = postman::Event {
            listen: postman::EventType::Test,
            script: postman::Script {
                exec: std::mem::replace(&mut self.body, Default::default()),
                script_type: "text/javascript",
            },
        };

        Some(event)
    }
}

impl<'a> From<&'a parser::Method> for postman::Method {
    fn from(value: &'a parser::Method) -> Self {
        match value {
            parser::Method::Get(_) => postman::Method::Get,
            parser::Method::Post(_) => postman::Method::Post,
            parser::Method::Delete(_) => postman::Method::Delete,
            parser::Method::Put(_) => postman::Method::Put,
            parser::Method::Patch(_) => postman::Method::Patch,
            parser::Method::Options(_) => postman::Method::Options,
        }
    }
}
