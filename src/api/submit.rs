use anyhow::{Context, Result};
use serde_json::json;
use tracing::info;

use crate::api::send_request::send_api_request;
use crate::app::models::Question;
use crate::app::workflow::QuestionCtx;

/// 提交标题题目（直接提交，不需要搜索和匹配）
///
/// # 参数
/// - `question`: 题目数据
/// - `ctx`: 题目上下文
///
/// # 返回
/// - 提交成功或失败
pub async fn submit_title_question(question: &Question, ctx: &QuestionCtx) -> Result<()> {
    let prefix = ctx.log_prefix();
    info!("{} 提交标题题目", prefix);

    let body_data = json!({
        "paperId": ctx.paper_id,
        "inputType": 1,
        "questionIndex": ctx.question_index,
        "questionType": "2",
        "addFlag": 1,
        "sysCode": 1,
        "relationType": 0,
        "questionSource": 3,
        "structureType": "biaoti",
        "questionInfo": {
            "stem": format!("<span>{}</span>", question.stem)
        }
    });

    let url = "https://tps-tiku-api.staff.xdf.cn/question/new/save";

    let response = send_api_request(url, &body_data)
        .await
        .with_context(|| format!("{} 提交标题题目失败", prefix))?;

    info!("{} 标题题目提交成功: {:?}", prefix, response);

    Ok(())
}

/// 提交匹配到的题目
///
/// # 参数
/// - `ctx`: 题目上下文
/// - `matched_question_id`: 匹配到的题目ID
/// - `search_source`: 搜索来源（"k12" 或 "xueke"）
///
/// # 返回
/// - 提交成功或失败
pub async fn submit_matched_question(
    ctx: &QuestionCtx,
    matched_question_id: &str,
    search_source: &str,
) -> Result<()> {
    let prefix = ctx.log_prefix();
    info!(
        "{} 提交匹配题目，ID: {}, 来源: {}",
        prefix, matched_question_id, search_source
    );

    let body_data = json!({
        "paperId": ctx.paper_id,
        "inputType": 1,
        "questionIndex": ctx.question_index,
        "questionType": "2",
        "addFlag": 1,
        "sysCode": 1,
        "relationType": 1, // 关联题目
        "questionSource": 2,
        "questionId": matched_question_id,
    });

    let url = "https://tps-tiku-api.staff.xdf.cn/question/new/save";

    let response = send_api_request(url, &body_data)
        .await
        .with_context(|| format!("{} 提交匹配题目失败", prefix))?;

    info!("{} 匹配题目提交成功: {:?}", prefix, response);

    Ok(())
}
