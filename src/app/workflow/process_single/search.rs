use crate::api::search::SearchResult;

/// 搜索策略结果
#[allow(dead_code)]
pub struct SearchStrategyResult {
    /// 搜索到的结果列表
    pub results: Vec<SearchResult>,
    /// 使用的搜索源（"k12" 或 "xueke"）
    pub source: String,
}

