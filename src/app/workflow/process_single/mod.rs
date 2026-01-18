//! 单题处理流程
//!
//! 完整流程：上传 → 搜索 → LLM 匹配 → 提交

mod search;
mod upload;

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::api::llm::service::LlmService;
use crate::api::search::{k12::k12_search, xueke::xueke_search};
pub use crate::api::submit::submit_title_question;
pub use crate::api::submit::submit_matched_question;
use crate::app::models::Question;
use crate::app::workflow::QuestionCtx;
use crate::app::workflow::process_single::upload::upload_screenshot_tohaoran;
use crate::config::AppConfig;

// use search::search_with_strategy; 
// use upload::upload_screenshot;

/// 单题处理结果
#[derive(Debug)]
pub struct ProcessResult {
    /// 是否找到匹配的原题
    pub found_match: bool,
    /// 匹配的题目索引（如果找到）
    pub matched_index: Option<usize>,
    /// 搜索源（"k12" 或 "xueke"）
    pub search_source: Option<String>,
    /// 匹配题目的原始数据
    pub matched_data: Option<serde_json::Value>,
    /// 上传的截图 URL
    pub screenshot_url: String,
}


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
    let screenshot_url = upload_screenshot_tohaoran(&ctx)
        .await
        .with_context(|| format!("{} 上传截图失败", prefix))?;

    // === 2. 搜索与匹配 (策略：K12 -> LLM Check -> if fail -> Xueke -> LLM Check) ===
    let mut final_matched_index = None;
    let mut final_search_results = Vec::new();
    let mut search_source = None;

    // --- 尝试 K12 ---
    info!("{} [步骤 2.1] 尝试 K12 搜索", prefix);
    let k12_success = match k12_search(&ctx.stage, &ctx.subject_code, ocr_text).await {
        Ok(results) if !results.is_empty() => {
             info!("{} K12 搜索找到 {} 条结果，开始 LLM 匹配", prefix, results.len());
             match llm_service.find_best_match_index(&results, &screenshot_url).await {
                 Ok(Some(idx)) => {
                     info!("{} K12 结果匹配成功！索引: {}", prefix, idx);
                     final_matched_index = Some(idx);
                     final_search_results = results;
                     search_source = Some("k12".to_string());
                     true
                 },
                 Ok(None) => {
                     info!("{} K12 结果经 LLM 判断均不匹配", prefix);
                     false
                 },
                 Err(e) => {
                     warn!("{} K12 结果 LLM 匹配过程出错: {:?}", prefix, e);
                     false
                 }
             }
        },
        Ok(_) => {
            info!("{} K12 搜索无结果", prefix);
            false
        },
        Err(e) => {
            warn!("{} K12 搜索请求失败: {:?}", prefix, e);
            false
        }
    };

    // --- 如果 K12 失败，尝试学科网 ---
    if !k12_success {
        info!("{} [步骤 2.2] K12 未匹配，尝试学科网搜索", prefix);
        // 学科网搜索比较贵，只有在其前面步骤失败时才调用
        match xueke_search(&ctx.stage, &ctx.subject_code, None, Some(ocr_text)).await {
            Ok(results) if !results.is_empty() => {
                info!("{} 学科网搜索找到 {} 条结果，开始 LLM 匹配", prefix, results.len());
                match llm_service.find_best_match_index(&results, &screenshot_url).await {
                    Ok(Some(idx)) => {
                        info!("{} 学科网结果匹配成功！索引: {}", prefix, idx);
                        final_matched_index = Some(idx);
                        final_search_results = results;
                        search_source = Some("xueke".to_string());
                    },
                    Ok(None) => {
                        info!("{} 学科网结果经 LLM 判断均不匹配", prefix);
                    },
                     Err(e) => {
                         warn!("{} 学科网结果 LLM 匹配过程出错: {:?}", prefix, e);
                     }
                }
            },
           Ok(_) => {
                info!("{} 学科网搜索无结果", prefix);
            },
            Err(e) => {
                warn!("{} 学科网搜索请求失败: {:?}", prefix, e);
            }
        }
    }

    // === 3. 构建结果 ===
    match final_matched_index {
        Some(idx) => {
             // 获取匹配题目的原始数据
             let matched_data = final_search_results.get(idx).map(|r| r.raw_data.clone());
             
             Ok(ProcessResult {
                found_match: true,
                matched_index: Some(idx),
                search_source,
                matched_data,
                screenshot_url,
            })
        },
        None => {
            info!("{} 最终未找到匹配题目", prefix);
            Ok(ProcessResult {
                found_match: false,
                matched_index: None,
                search_source: None,
                matched_data: None,
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
            is_title: false,
            screenshot: question.screenshot.clone(),
        };

        let result = process_single_question(&question, &ctx, &llm_service, "测试文本")
            .await
            .expect("处理失败");

        println!("处理结果: {:?}", result);
    }
}
