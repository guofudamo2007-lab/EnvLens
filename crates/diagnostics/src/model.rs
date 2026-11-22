use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Text {
    pub zh: String,
    pub en: String,
}
pub type Pair<'a> = (&'a str, &'a str);
impl From<Pair<'_>> for Text {
    fn from((zh, en): Pair) -> Self {
        Self {
            zh: zh.into(),
            en: en.into(),
        }
    }
}
impl Text {
    pub fn get(&self, english: bool) -> &str {
        if english {
            &self.en
        } else {
            &self.zh
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Warning,
    Error,
    Skipped,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    System,
    Gpu,
    Python,
    Torch,
    Ollama,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Confirmed,
    Likely,
    Undetermined,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Evidence {
    pub key: String,
    pub value: String,
    pub private: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Finding {
    pub confidence: Confidence,
    pub severity: String,
    pub impact: Text,
    pub cause: Text,
    pub steps: Vec<Text>,
    pub commands: Vec<String>,
    pub docs: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Check {
    pub id: String,
    pub group: Group,
    pub title: Text,
    pub status: Status,
    pub summary: Text,
    pub evidence: Vec<Evidence>,
    pub finding: Option<Finding>,
}
impl Check {
    pub fn new(id: &str, group: Group, title: Pair, status: Status, summary: Pair) -> Self {
        Self {
            id: id.into(),
            group,
            title: title.into(),
            status,
            summary: summary.into(),
            evidence: vec![],
            finding: None,
        }
    }
    pub fn fact(mut self, key: &str, value: impl ToString) -> Self {
        self.evidence.push(Evidence {
            key: key.into(),
            value: value.to_string(),
            private: false,
        });
        self
    }
    pub fn private(mut self, key: &str, value: impl ToString) -> Self {
        self.evidence.push(Evidence {
            key: key.into(),
            value: value.to_string(),
            private: true,
        });
        self
    }
    pub fn explain(
        mut self,
        confidence: Confidence,
        impact: Pair,
        cause: Pair,
        steps: &[Pair],
        docs: &[&str],
    ) -> Self {
        self.finding = Some(Finding {
            confidence,
            severity: match self.status {
                Status::Error => "high",
                Status::Warning => "medium",
                _ => "info",
            }
            .into(),
            impact: impact.into(),
            cause: cause.into(),
            steps: steps.iter().copied().map(Into::into).collect(),
            commands: vec![],
            docs: docs.iter().map(|s| s.to_string()).collect(),
        });
        self
    }
    pub fn command(mut self, command: String) -> Self {
        if let Some(f) = &mut self.finding {
            f.commands.push(command);
        }
        self
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub id: String,
    pub started_at: String,
    pub duration_ms: u64,
    pub interpreter: Option<String>,
    pub project: Option<String>,
    pub redacted: bool,
    pub checks: Vec<Check>,
}
pub const PY_DOC: &str = "https://docs.python.org/3/using/windows.html";
pub const VENV_DOC: &str = "https://docs.python.org/3/library/venv.html";
pub const PIP_DOC: &str = "https://pip.pypa.io/en/stable/cli/pip_check/";
pub const TORCH_DOC: &str = "https://pytorch.org/get-started/locally/";
pub const CUDA_DOC: &str =
    "https://docs.nvidia.com/deploy/cuda-compatibility/minor-version-compatibility.html";
pub const GPU_DOC: &str = "https://docs.nvidia.com/deploy/nvidia-smi/";
pub const OLLAMA_DOC: &str = "https://docs.ollama.com/windows";
pub const TAGS_DOC: &str = "https://docs.ollama.com/api/tags";
pub const DOCS: &[&str] = &[
    PY_DOC, VENV_DOC, PIP_DOC, TORCH_DOC, CUDA_DOC, GPU_DOC, OLLAMA_DOC, TAGS_DOC,
];

pub fn ps_python(path: &str, args: &str) -> String {
    format!("& '{}' {args}", path.replace('\'', "''"))
}
