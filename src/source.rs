use std::{fs, path::Path};

use anyhow::Context;

use crate::{
    parser::{parse, File, RequestScript},
    Result,
};

#[derive(Debug)]
pub struct SourceItem<'a> {
    pub name: &'a str,
    pub index: usize,
    pub script: &'a RequestScript,
}

pub trait SourceProvider {
    fn requests(&mut self) -> impl Iterator<Item = SourceItem>;
}

pub struct FileSourceProvider(File, String, Option<usize>);

impl FileSourceProvider {
    pub fn new(file: impl AsRef<Path>, request: Option<usize>) -> Result<Self> {
        let name = file.as_ref().display().to_string();

        let file_contents = fs::read_to_string(&file)
            .with_context(|| format!("Failed opening script file: `{}`", name))?;

        let file = parse(file.as_ref().to_path_buf(), file_contents.as_str())
            .with_context(|| format!("Failed parsing file: `{}`", name))?;

        Ok(Self(file, name, request))
    }
}

impl SourceProvider for FileSourceProvider {
    fn requests(&mut self) -> impl Iterator<Item = SourceItem> {
        self.0
            .request_scripts(self.2)
            .map(|(index, script)| SourceItem {
                name: &self.1,
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
