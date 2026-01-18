use crate::app::models::Question;

#[derive(Debug, Clone, Copy)]
pub enum SearchSource {
    K12,
    Xueke,
}

impl SearchSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchSource::K12 => "k12",
            SearchSource::Xueke => "xueke",
        }
    }
}

/// 统一的出口协议，覆盖“找到”“生成”“需人工”三类结果。
#[derive(Debug)]
pub enum BuildResult {
    Found {
        source: SearchSource,
        matched_index: usize,
        matched_data: serde_json::Value,
        screenshot_url: String,
    },
    Generated {
        question: Question,
        screenshot_url: String,
    },
    ManualRequired {
        paper_id: String,
        index: usize,
        screenshot_url: String,
        reason: String,
    },
}

#[derive(Debug)]
pub enum StepError {
    NotFound,
    LlmRejected,
    RetryExhausted,
    UnsupportedQuestion,
    LlmBuildFailed(String),
    InfraError(anyhow::Error),
}

impl From<anyhow::Error> for StepError {
    fn from(err: anyhow::Error) -> Self {
        StepError::InfraError(err)
    }
}

impl StepError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, StepError::NotFound | StepError::LlmRejected)
    }
}