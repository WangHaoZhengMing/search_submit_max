use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum SearchSource {
    K12,
    Xueke,
}



/// 统一的出口协议，覆盖“找到”“生成”“需人工”三类结果。
#[derive(Debug)]
#[allow(dead_code)]
pub enum BuildResult {
    Found {
        matched_data: Value,
    },
    Generated {
        question: Value,
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
    UnsupportedQuestion,
    LlmBuildFailed(String),
    InfraError(anyhow::Error),
}

impl From<anyhow::Error> for StepError {
    fn from(err: anyhow::Error) -> Self {
        StepError::InfraError(err)
    }
}

