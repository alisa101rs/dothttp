use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TestsReport {
    #[serde(flatten)]
    tests: BTreeMap<String, TestResult>,
}

impl TestsReport {
    pub fn failed(&self) -> impl Iterator<Item = (&String, &TestResult)> {
        self.tests.iter().filter(|(_, r)| r.is_error())
    }
    pub fn all(&self) -> impl Iterator<Item = (&String, &TestResult)> {
        self.tests.iter()
    }
    pub fn is_empty(&self) -> bool {
        self.tests.is_empty()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "result")]
pub enum TestResult {
    #[serde(rename = "error")]
    Error { error: String },
    #[serde(rename = "success")]
    Success,
}

impl TestResult {
    pub fn is_error(&self) -> bool {
        matches!(self, TestResult::Error { .. })
    }
}
