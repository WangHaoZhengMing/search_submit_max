//! 单题处理流程
//!
//! 完整流程：上传 → 搜索 → LLM 匹配 → 提交

mod search;
mod upload;

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::api::llm::service::LlmService;
pub use crate::api::submit::submit_title_question;
use crate::app::models::Question;
use crate::app::workflow::QuestionCtx;
use crate::config::AppConfig;

use search::search_with_strategy;
use upload::upload_screenshot;

/// 单题处理结果
#[derive(Debug)]
pub struct ProcessResult {
    /// 是否找到匹配的原题
    pub found_match: bool,
    /// 匹配的题目索引（如果找到）
    pub matched_index: Option<usize>,
    /// 搜索源（"k12" 或 "xueke"）
    pub search_source: Option<String>,
    /// 上传的截图 URL
    pub screenshot_url: String,
}

/// 处理单个题目的完整流程
///
/// # 参数
/// - `question`: 题目数据
/// - `ctx`: 题目上下文
/// - `llm_service`: LLM 服务实例
/// - `ocr_text`: OCR 识别的文本
///
/// # 返回
/// - 处理结果
pub async fn process_single_question(
    _question: &Question,
    ctx: &QuestionCtx,
    llm_service: &LlmService,
    ocr_text: &str,
) -> Result<ProcessResult> {
    let prefix = ctx.log_prefix();
    info!("{} ========== 开始处理题目 ==========", prefix);

    // === 1. 上传截图 ===
    info!("{} [步骤 1/3] 上传截图", prefix);
    let screenshot_url = upload_screenshot(&ctx)
        .await
        .with_context(|| format!("{} 上传截图失败", prefix))?;

    // === 2. 搜索题库 ===
    info!("{} [步骤 2/3] 搜索题库", prefix);
    let search_result = match search_with_strategy(&ctx, ocr_text).await {
        Ok(result) => result,
        Err(e) => {
            warn!("{} 搜索失败: {}，跳过 LLM 匹配", prefix, e);
            return Ok(ProcessResult {
                found_match: false,
                matched_index: None,
                search_source: None,
                screenshot_url,
            });
        }
    };

    info!(
        "{} 使用 {} 搜索，找到 {} 个候选题目",
        prefix,
        search_result.source,
        search_result.results.len()
    );

    // === 3. LLM 匹配 ===
    info!("{} [步骤 3/3] LLM 匹配", prefix);
    let matched_index = llm_service
        .find_best_match_index(&search_result.results, &screenshot_url)
        .await
        .with_context(|| format!("{} LLM 匹配失败", prefix))?;

    match matched_index {
        Some(idx) => {
            info!("{} ✓ 找到匹配题目，索引: {}", prefix, idx);
            Ok(ProcessResult {
                found_match: true,
                matched_index: Some(idx),
                search_source: Some(search_result.source),
                screenshot_url,
            })
        }
        None => {
            info!("{} ✗ 未找到匹配题目", prefix);
            Ok(ProcessResult {
                found_match: false,
                matched_index: None,
                search_source: Some(search_result.source),
                screenshot_url,
            })
        }
    }
}

/// 创建 LLM 服务实例
pub fn create_llm_service(config: &AppConfig) -> LlmService {
    LlmService::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::logger;

    #[tokio::test]
    #[ignore] // 需要真实配置才能运行
    async fn test_process_single_question() {
        logger::init_test();

        let config = AppConfig::load().expect("加载配置失败");
        let llm_service = create_llm_service(&config);

        let question = Question {
            origin: "测试".to_string(),
            stem: "测试题干".to_string(),
            origin_from_our_bank: vec![],
            is_title: false,
            imgs: None,
            screenshot: "data:image/png;base64,iVBORw0KGgo...".to_string(),
        };

        let ctx = QuestionCtx {
            paper_id: "test_paper_id".to_string(),
            subject_code: "61".to_string(),
            stage: "3".to_string(),
            paper_index: 1,
            question_index: 1,
            screenshot: question.screenshot.clone(),
        };

        let result = process_single_question(&question, &ctx, &llm_service, "测试文本")
            .await
            .expect("处理失败");

        println!("处理结果: {:?}", result);
    }
}
