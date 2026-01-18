//! 单题处理流程
//!
//! 完整流程：上传 → 搜索 → LLM 匹配 → 兜底

mod build_ques_llm;
mod search;
mod upload;
pub(crate) mod result;

use anyhow::{anyhow, Context, Result};
use tracing::{info, warn};

use crate::api::llm::service::LlmService;
pub use crate::api::submit::{submit_matched_question, submit_title_question};
use crate::app::models::Question;
use crate::app::workflow::QuestionCtx;
use crate::app::workflow::process_single::build_ques_llm::build_question_via_llm;
use crate::app::workflow::process_single::result::{BuildResult, SearchSource, StepError};
use crate::app::workflow::process_single::search::{k12_fallback, search_k12, MatchOutput};
use crate::app::workflow::process_single::upload::upload_screenshot_tohaoran;
use crate::config::AppConfig;

/// 单题处理主流程：上传 → 搜索（主链路）→ 搜索（fallback）→ LLM 构建 → 人工兜底
pub async fn process_single_question(
    _question: &Question,
    ctx: &QuestionCtx,
    llm_service: &LlmService,
    ocr_text: &str,
) -> Result<BuildResult> {
    let prefix = ctx.log_prefix();
    info!("{} ========== 开始处理题目 ==========" , prefix);

    // === 1. 上传截图 ===
    info!("{} [步骤 1/3] 上传截图", prefix);
    let screenshot_url = upload_screenshot_tohaoran(&ctx)
        .await
        .with_context(|| format!("{} 上传截图失败", prefix))?;

    // === 2. 搜索与匹配 (K12 -> 学科网) ===
    match search_k12(ctx, llm_service, ocr_text, &screenshot_url).await {
        Ok(found) => return Ok(to_build_result(found, screenshot_url)),
        Err(e) => {
            log_step_error(&prefix, SearchSource::K12, &e);
            if matches!(e, StepError::InfraError(_) | StepError::RetryExhausted) {
                return Err(anyhow!("{} 主链路失败: {:?}", prefix, e));
            }
        }
    }

    info!("{} [步骤 2/3] K12 未命中，进入 fallback", prefix);

    match k12_fallback(ctx, llm_service, ocr_text, &screenshot_url).await {
        Ok(found) => return Ok(to_build_result(found, screenshot_url)),
        Err(e) => {
            log_step_error(&prefix, SearchSource::Xueke, &e);
            if matches!(e, StepError::InfraError(_) | StepError::RetryExhausted) {
                return Err(anyhow!("{} fallback 阶段失败: {:?}", prefix, e));
            }
        }
    }

    // === 3. LLM 构建 / 人工兜底 ===
    info!("{} [步骤 3/3] 搜索链路未命中，尝试 LLM 构建", prefix);
    match build_question_via_llm(ctx, ocr_text, &screenshot_url).await {
        Ok(question) => {
            info!("{} LLM 构建成功，标记为 Generated", prefix);
            Ok(BuildResult::Generated { question, screenshot_url })
        }
        Err(e) => {
            warn!(
                target: "failed_questions",
                "{} 搜索与构建均未命中，人工处理 | paper_id={} | idx={} | reason={:?}",
                prefix,
                ctx.paper_id,
                ctx.not_include_title_index,
                e
            );
            Ok(BuildResult::ManualRequired {
                paper_id: ctx.paper_id.clone(),
                index: ctx.not_include_title_index,
                screenshot_url,
                reason: format!("{:?}", e),
            })
        }
    }
}

fn to_build_result(found: MatchOutput, screenshot_url: String) -> BuildResult {
    BuildResult::Found {
        source: found.source,
        matched_index: found.matched_index,
        matched_data: found.matched_data,
        screenshot_url,
    }
}

fn log_step_error(prefix: &str, source: SearchSource, err: &StepError) {
    match err {
        StepError::InfraError(e) => warn!("{} {:?} 请求异常: {:?}", prefix, source, e),
        StepError::RetryExhausted => warn!("{} {:?} 重试耗尽", prefix, source),
        other => info!("{} {:?} 未命中，原因: {:?}", prefix, source, other),
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
            not_include_title_index: 1,
        };

        let result = process_single_question(&question, &ctx, &llm_service, "测试文本")
            .await
            .expect("处理失败");

        match result {
            BuildResult::Found { source, matched_index, .. } => {
                println!("匹配成功 | source={:?} | idx={}", source, matched_index);
            }
            BuildResult::Generated { .. } => {
                println!("LLM 构建成功");
            }
            BuildResult::ManualRequired { .. } => {
                println!("需人工介入");
            }
        }
    }
}
