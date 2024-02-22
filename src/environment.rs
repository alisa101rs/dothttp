use std::{
    fs, io,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{anyhow, Context};
use serde_json::Value;

pub trait EnvironmentProvider {
    fn snapshot(&self) -> Value;
    fn save(&mut self, snapshot: &Value) -> io::Result<()>;
}

#[derive(Debug, Clone)]
pub struct StaticEnvironmentProvider {
    env: Value,
    snapshot: Option<Value>,
}

impl StaticEnvironmentProvider {
    pub fn new(env: Value) -> Self {
        Self {
            env,
            snapshot: None,
        }
    }

    pub fn snapshot(&self) -> &serde_json::Map<String, Value> {
        self.snapshot
            .as_ref()
            .expect("snapshot to be set")
            .as_object()
            .expect("snapshot to be object")
    }
}

impl EnvironmentProvider for StaticEnvironmentProvider {
    fn snapshot(&self) -> Value {
        self.env.clone()
    }

    fn save(&mut self, snapshot: &Value) -> io::Result<()> {
        self.snapshot = Some(snapshot.clone());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EnvironmentFileProvider {
    snapshot: serde_json::Map<String, Value>,
    snapshot_path: PathBuf,
}

impl EnvironmentFileProvider {
    pub fn open(
        environment_name: &str,
        environment_path: impl AsRef<Path>,
        snapshot_path: impl AsRef<Path>,
    ) -> crate::Result<Self> {
        let Value::Object(mut environment) =
            read_json_content(environment_path.as_ref()).context("environment deserialization")?
        else {
            return Err(anyhow!("Expected environment file to be a map"));
        };

        let Value::Object(mut environment) = environment
            .remove(environment_name)
            .unwrap_or_else(|| serde_json::json!({}))
        else {
            return Err(anyhow!("Expected selected environment to be a map"));
        };

        let Value::Object(mut snapshot) =
            read_json_content(snapshot_path.as_ref()).context("snapshot deserialization")?
        else {
            return Err(anyhow!("Expected snapshot file to be a map"));
        };

        snapshot.append(&mut environment);

        Ok(Self {
            snapshot,
            snapshot_path: snapshot_path.as_ref().to_owned(),
        })
    }
}

impl EnvironmentProvider for EnvironmentFileProvider {
    fn snapshot(&self) -> Value {
        Value::Object(self.snapshot.clone())
    }

    fn save(&mut self, snapshot: &Value) -> io::Result<()> {
        fs::write(&self.snapshot_path, serde_json::to_string_pretty(snapshot)?)?;

        Ok(())
    }
}

fn read_json_content(path: &Path) -> crate::Result<Value> {
    match fs::read(path) {
        Ok(data) => Ok(serde_json::from_slice(&data).context("json deserialization")?),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(e) => Err(anyhow!("IO Error: {e}")),
    }
}
