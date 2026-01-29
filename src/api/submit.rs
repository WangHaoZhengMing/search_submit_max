use anyhow::{Context, Result};
use serde_json::json;
use tracing::{debug, info};

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
/// - `matched_data`: 匹配到的完整题目数据 (serde_json::Value)
/// - `search_source`: 搜索来源（"k12" 或 "xueke"）
///
/// # 返回
/// - 提交成功或失败
pub async fn submit_matched_question(
    ctx: &QuestionCtx,
    matched_data: &serde_json::Value,
) -> Result<()> {
    let prefix = ctx.log_prefix();

    // 基于匹配到的题目数据构建请求体
    let mut body_data = matched_data.clone();

    // 填充提交所需的上下文信息
    body_data["paperId"] = json!(ctx.paper_id);
    body_data["inputType"] = json!(1);
    body_data["questionIndex"] = json!(ctx.question_index);
    body_data["questionType"] = json!("1");
    body_data["addFlag"] = json!(1);
    body_data["sysCode"] = json!(1);
    body_data["relationType"] = json!(1);
    body_data["questionSource"] = json!(2);

    let url = "https://tps-tiku-api.staff.xdf.cn/question/new/save";

    let response = send_api_request(url, &body_data)
        .await
        .with_context(|| format!("{} 提交匹配题目失败", prefix))?;

    debug!("{} 匹配题目提交成功: {:?}", prefix, response);

    Ok(())
}

pub async fn submit_generated_question(
    ctx: &QuestionCtx,
    matched_data: &serde_json::Value,
) -> Result<()> {
    let url = "https://tps-tiku-api.staff.xdf.cn/question/new/save";
    let prefix = ctx.log_prefix();
    // 基于匹配到的题目数据构建请求体
    let body_data = matched_data.clone();
    let response = send_api_request(url, &body_data)
        .await
        .with_context(|| format!("{} 提交匹配题目失败", prefix))?;
    info!("{} LLM生成题目提交成功: {:?}", prefix, response);

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要真实环境网络
    async fn test_submit_matched_question() {
        let ctx = QuestionCtx {
            paper_id: "3425387499337388032".to_string(),
            subject_code: "54".to_string(),
            stage: "3".to_string(),
            paper_index: 1,
            question_index: 2,
            is_title: false,
            screenshot: "".to_string(),
            not_include_title_index: 1,
        };

        // 模拟搜索到的原始数据 (基于提供的示例)
        // 使用 from_str 避免 json! 宏的递归深度限制
        let matched_data: serde_json::Value = json!(
            {
  "addFlag": 1,
  "businessType": "CSX-DANXUAN",
  "inputType": 1,
  "paperId": "3425387499337388032",
  "questionIndex": 1,
  "questionInfo": {
    "analysis": "<p>本题考查了中心对称图形与 轴对称图形的概念。\n轴对称图形的关键是寻找对称轴，图形两部分折叠后可重合；\n中心对称图形是要寻找对称中心，旋转 $180^{\\circ}$ 后与原图重合。\n解：A．是轴对称图形，不是中心对称图形，故本选项不符合题意；\nB．是轴对称图形，不是中心对称图形，故本选项不符合题意；\nC．既是轴对称图形，又是中心对称图形，故本选项符合题意；\nD．是轴对称图形，不是中心对称图形，故本选项不符合题意。\n故选：C．\n</p>\n",
    "answer": "D",
    "options": [
      {
        "flagAnswer": "0",
        "htmlCode": "<p>  <span style=\"visibility: hidden;\">.</span>  <svg width=\"100\" height=\"100\" viewBox=\"0 0 100 100\">\n      <path d=\"M20 75 H80 V45 Q80 35 50 15 Q20 35 20 45 Z\" fill=\"#D3D3D3\" stroke=\"black\" stroke-width=\"1\"/>\n    </svg>\n    </p>\n",
        "optCode": "ab00",
        "title": "A"
      },
      {
        "flagAnswer": "0",
        "htmlCode": "<p>  <span style=\"visibility: hidden;\">.</span>  <svg width=\"100\" height=\"100\" viewBox=\"0 0 100 100\">\n      <path d=\"M30 20 Q50 10 70 20 Q60 50 70 80 Q50 90 30 80 Q40 50 30 20\" fill=\"#D3D3D3\" stroke=\"black\" stroke-width=\"1\"/>\n    </svg>\n    </p>\n",
        "optCode": "ab01",
        "title": "B"
      },
      {
        "flagAnswer": "1",
        "htmlCode": "<p>  <span style=\"visibility: hidden;\">.</span>  <svg width=\"100\" height=\"100\" viewBox=\"0 0 100 100\">\n      <path d=\"M35 15 H65 V35 H85 V65 H65 V85 H35 V65 H15 V35 H35 Z\" fill=\"#D3D3D3\" stroke=\"black\" stroke-width=\"1\"/>\n    </svg>\n    </p>\n",
        "optCode": "ab02",
        "title": "C"
      },
      {
        "flagAnswer": "0",
        "htmlCode": "<p>  <span style=\"visibility: hidden;\">.</span>  <svg width=\"100\" height=\"100\" viewBox=\"0 0 100 100\">\n      <circle cx=\"50\" cy=\"50\" r=\"45\" fill=\"none\" stroke=\"black\" stroke-width=\"1\"/>\n      <path d=\"M50 5 L61 38 H95 L68 58 L78 90 L50 70 L22 90 L32 58 L5 38 H39 Z\" fill=\"none\" stroke=\"black\" stroke-width=\"1\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"50\" y2=\"5\" stroke=\"black\" stroke-width=\"0.5\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"61\" y2=\"38\" stroke=\"black\" stroke-width=\"0.5\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"95\" y2=\"38\" stroke=\"black\" stroke-width=\"0.5\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"68\" y2=\"58\" stroke=\"black\" stroke-width=\"0.5\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"78\" y2=\"90\" stroke=\"black\" stroke-width=\"0.5\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"50\" y2=\"70\" stroke=\"black\" stroke-width=\"0.5\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"22\" y2=\"90\" stroke=\"black\" stroke-width=\"0.5\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"32\" y2=\"58\" stroke=\"black\" stroke-width=\"0.5\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"5\" y2=\"38\" stroke=\"black\" stroke-width=\"0.5\"/>\n      <line x1=\"50\" y1=\"50\" x2=\"39\" y2=\"38\" stroke=\"black\" stroke-width=\"0.5\"/>\n    </svg>\n    </p>\n",
        "optCode": "ab03",
        "title": "D"
      }
    ],
    "stem": "<p>下面四幅图案，其中既是轴对称图形又是中心对称图形的是（　　）\n （&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;）</p>\n"
  },
  "questionSource": 2,
  "questionType": 1,
  "relationType": 0,
  "structureType": "danxuan",
  "sysCode": 1
}        );

        println!("开始测试提交匹配题目...");
        let result = submit_matched_question(&ctx, &matched_data).await;

        match result {
            Ok(_) => println!("提交成功!"),
            Err(e) => {
                println!("提交失败: {:?}", e);
                // 这里不 panic，因为这是网络请求，本地测试可能会失败
            }
        }
    }
}
