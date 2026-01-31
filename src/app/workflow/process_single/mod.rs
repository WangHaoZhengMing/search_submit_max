mod build_ques_llm;
pub(crate) mod result;
mod search;
mod upload;
use crate::api::llm::service::LlmService;
pub use crate::api::submit::{submit_matched_question, submit_title_question};
use crate::app::models::Question;
use crate::app::workflow::QuestionCtx;
use crate::app::workflow::process_single::build_ques_llm::build_question_via_llm;
use crate::app::workflow::process_single::result::BuildResult;
use crate::app::workflow::process_single::search::{MatchOutput, k12_fallback, search_k12};
use crate::app::workflow::process_single::upload::{upload_screenshot,};
use crate::config::AppConfig;
use anyhow::{Context, Result};
use tracing::{info, warn};

pub async fn process_single_question(
    _question: &Question,
    ctx: &QuestionCtx,
    llm_service: &LlmService,
    ocr_text: &str,
) -> Result<BuildResult> {
    let prefix = ctx.log_prefix();
    info!("{} [步骤 1/3] 上传截图", prefix);
    let screenshot_url = upload_screenshot(&ctx)
        .await
        .with_context(|| format!("{} 上传截图失败", prefix))?;

    match search_k12(ctx, llm_service, ocr_text, &screenshot_url).await {
        Ok(found) => return Ok(to_build_result(found)),
        Err(e) => {
            tracing::warn!("{} K12 搜索与匹配失败: {:?}", prefix, e);
        }
    }

    info!("{} [步骤 2/3] K12 未命中，学科网", prefix);
    match k12_fallback(ctx, llm_service, ocr_text, &screenshot_url).await {
        Ok(found) => return Ok(to_build_result(found)),
        Err(e) => {
            tracing::warn!("{} 学科搜索与匹配未命中: {:?}", prefix, e);
        }
    }

    info!("{} [步骤 3/3] 搜索链路均未命中，尝试 LLM 构建", prefix);
    match build_question_via_llm(ctx, ocr_text, &screenshot_url).await {
        Ok(question_value) => {
            info!("{} LLM 构建成功", prefix);
            Ok(BuildResult::Generated {
                question: question_value,
                screenshot_url,
            })
        }
        Err(e) => {
            warn!(
                prefix,
                ctx.paper_id,
                ctx.not_include_title_index,
                ctx.screenshot
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

fn to_build_result(found: MatchOutput) -> BuildResult {
    BuildResult::Found {
        matched_data: found.matched_data,
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
            BuildResult::Found { .. } => {
                println!("匹配成功");
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
