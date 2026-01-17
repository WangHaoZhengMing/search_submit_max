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
}
