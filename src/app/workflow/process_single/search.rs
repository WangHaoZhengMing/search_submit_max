use anyhow::Context;
use serde_json::Value;
use tracing::{info, warn};

use crate::api::llm::service::LlmService;
use crate::api::search::{k12::k12_search, xueke::xueke_search, SearchResult};
use crate::app::workflow::QuestionCtx;
use crate::app::workflow::process_single::result::{SearchSource, StepError};

#[derive(Debug)]
pub struct MatchOutput {
	pub source: SearchSource,
	pub matched_index: usize,
	pub matched_data: Value,
}

async fn run_search_with_llm<F, Fut>(
	ctx: &QuestionCtx,
	source: SearchSource,
	screenshot_url: &str,
	llm_service: &LlmService,
	search_fn: F,
) -> Result<MatchOutput, StepError>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = anyhow::Result<Vec<SearchResult>>>,
{
	let prefix = ctx.log_prefix();
	info!("{} [search] 触发 {:?}", prefix, source);

	let results = search_fn()
		.await
		.with_context(|| format!("{} {:?} 搜索失败", prefix, source))
		.map_err(StepError::InfraError)?;

	if results.is_empty() {
		info!("{} {:?} 搜索无结果", prefix, source);
		return Err(StepError::NotFound);
	}

	let matched_index = llm_service
		.find_best_match_index(&results, screenshot_url)
		.await
		.map_err(StepError::InfraError)?;

	match matched_index {
		Some(idx) => {
			let matched_data = results
				.get(idx)
				.map(|r| r.raw_data.clone())
				.unwrap_or_else(|| Value::Null);

			info!("{} {:?} LLM 匹配成功，索引 {}", prefix, source, idx);

			Ok(MatchOutput {
				source,
				matched_index: idx,
				matched_data,
			})
		}
		None => {
			warn!("{} {:?} LLM 判断无匹配", prefix, source);
			Err(StepError::LlmRejected)
		}
	}
}

pub async fn search_k12(
	ctx: &QuestionCtx,
	llm_service: &LlmService,
	ocr_text: &str,
	screenshot_url: &str,
) -> Result<MatchOutput, StepError> {
	run_search_with_llm(ctx, SearchSource::K12, screenshot_url, llm_service, || {
		k12_search(&ctx.stage, &ctx.subject_code, ocr_text)
	})
	.await
}

/// 学科网搜索作为 fallback。
pub async fn k12_fallback(
	ctx: &QuestionCtx,
	llm_service: &LlmService,
	ocr_text: &str,
	screenshot_url: &str,
) -> Result<MatchOutput, StepError> {
	run_search_with_llm(ctx, SearchSource::Xueke, screenshot_url, llm_service, || {
		xueke_search(&ctx.stage, &ctx.subject_code, None, Some(ocr_text))
	})
	.await
}

