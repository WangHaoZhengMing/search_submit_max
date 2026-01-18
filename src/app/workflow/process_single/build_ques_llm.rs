use std::result::Result;

use crate::app::models::Question;
use crate::app::workflow::QuestionCtx;
use crate::app::workflow::process_single::result::StepError;

/// 预留的 LLM 构建题目逻辑。
/// 当前返回 UnsupportedQuestion，让上层走人工兜底。
pub async fn build_question_via_llm(
	_ctx: &QuestionCtx,
	_ocr_text: &str,
	_screenshot_url: &str,
) -> Result<Question, StepError> {
	Err(StepError::UnsupportedQuestion)
}
