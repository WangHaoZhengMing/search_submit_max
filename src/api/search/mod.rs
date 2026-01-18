use serde::{Deserialize, Serialize};

pub mod k12;
pub mod xueke;

/// 题库搜索结果数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    #[serde(rename = "questionContent")]
    pub question_content: String,

    #[serde(rename = "xkwQuestionSimilarity")]
    pub xkw_question_similarity: Option<f64>,

    pub img_urls: Option<Vec<String>>,

    #[serde(skip)]
    pub raw_data: serde_json::Value,
}

impl Default for SearchResult {
    fn default() -> Self {
        Self {
            question_content: String::new(),
            xkw_question_similarity: None,
            img_urls: None,
            raw_data: serde_json::Value::Null,
        }
    }
}
