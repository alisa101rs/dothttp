use std::{fs, path::Path};

use crate::parser::{parse, File, RequestScript};

use miette::{Context, IntoDiagnostic, NamedSource, Result};

#[derive(Debug, Copy, Clone)]
pub struct SourceItem<'a> {
    pub name: &'a str,
    pub index: usize,
    pub script: &'a RequestScript,
}

impl SourceItem<'_> {
    pub fn source_name(&self) -> &str {
        self.name
    }

    pub fn request_name(&self) -> String {
        if let Some(name) = &self.script.name {
            name.clone()
        } else {
            format!("#{}", self.index + 1)
        }
    }
}

pub trait SourceProvider {
    fn requests(&mut self) -> impl Iterator<Item = SourceItem>;
}

pub struct FileSourceProvider {
    file: File,
    name: String,
    request_index: Option<usize>,
}

impl FileSourceProvider {
    pub fn new(file: impl AsRef<Path>, request_index: Option<usize>) -> Result<Self> {
        let name = file.as_ref().display().to_string();

        let file_contents = fs::read_to_string(&file)
            .into_diagnostic()
            .with_context(|| format!("Failed opening script file: `{}`", name))?;

        let file = parse(file_contents.as_str()).map_err(|er| {
            er.with_source_code(NamedSource::new(
                file.as_ref().display().to_string(),
                file_contents,
            ))
        })?;
        Ok(Self {
            file,
            name,
            request_index,
        })
    }
}

impl SourceProvider for FileSourceProvider {
    fn requests(&mut self) -> impl Iterator<Item = SourceItem> {
        self.file
            .request_scripts(self.request_index)
            .map(|(index, script)| SourceItem {
                name: &self.name,
                index,
                script,
            })
    }
}

pub struct FilesSourceProvider(Vec<FileSourceProvider>);

impl FilesSourceProvider {
    pub fn from_list<T>(files: impl IntoIterator<Item = T>) -> Result<Self>
    where
        T: AsRef<str>,
    {
        let mut inner = vec![];
        for file in files {
            let (path, index) = if file.as_ref().contains('#') {
                file.as_ref().split_once('#').unwrap()
            } else {
                (file.as_ref(), "")
            };
            let request = index.parse().ok();
            inner.push(FileSourceProvider::new(path, request)?);
        }

        Ok(Self(inner))
    }
}

impl SourceProvider for FilesSourceProvider {
    fn requests(&mut self) -> impl Iterator<Item = SourceItem> {
        self.0.iter_mut().flat_map(|it| it.requests())
    }
}
