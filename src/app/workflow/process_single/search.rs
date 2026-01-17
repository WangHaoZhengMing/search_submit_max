use anyhow::Result;
use tracing::{info, warn};

use crate::api::search::{SearchResult, k12::k12_search, xueke::xueke_search};
use crate::app::workflow::QuestionCtx;

/// 搜索策略结果
pub struct SearchStrategyResult {
    /// 搜索到的结果列表
    pub results: Vec<SearchResult>,
    /// 使用的搜索源（"k12" 或 "xueke"）
    pub source: String,
}

/// 执行搜索策略：先 K12，失败后再 Xueke
///
/// # 参数
/// - `ctx`: 题目上下文
/// - `ocr_text`: OCR 识别的文本（用于搜索）
///
/// # 返回
/// - 搜索结果列表和使用的搜索源
pub async fn search_with_strategy(
    ctx: &QuestionCtx,
    ocr_text: &str,
) -> Result<SearchStrategyResult> {
    let prefix = ctx.log_prefix();

    // 1. 尝试 K12 搜索
    info!("{} 开始 K12 题库搜索", prefix);

    match k12_search(&ctx.stage, &ctx.subject_code, ocr_text).await {
        Ok(results) if !results.is_empty() => {
            info!("{} K12 搜索成功，找到 {} 条结果", prefix, results.len());
            return Ok(SearchStrategyResult {
                results,
                source: "k12".to_string(),
            });
        }
        Ok(_) => {
            warn!("{} K12 搜索返回空结果", prefix);
        }
        Err(e) => {
            warn!("{} K12 搜索失败: {}", prefix, e);
        }
    }

    // 2. K12 失败，尝试学科网搜索
    info!("{} K12 搜索无结果，尝试学科网搜索", prefix);

    match xueke_search(&ctx.stage, &ctx.subject_code, None, Some(ocr_text)).await {
        Ok(results) if !results.is_empty() => {
            info!("{} 学科网搜索成功，找到 {} 条结果", prefix, results.len());
            Ok(SearchStrategyResult {
                results,
                source: "xueke".to_string(),
            })
        }
        Ok(_) => {
            warn!("{} 学科网搜索返回空结果", prefix);
            Err(anyhow::anyhow!("{} 所有搜索策略均未找到结果", prefix))
        }
        Err(e) => {
            warn!("{} 学科网搜索失败: {}", prefix, e);
            Err(anyhow::anyhow!("{} 所有搜索策略均失败", prefix))
        }
    }
}
