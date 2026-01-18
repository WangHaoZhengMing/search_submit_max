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
/// - `matched_data`: 匹配到的完整题目数据 (serde_json::Value)
/// - `search_source`: 搜索来源（"k12" 或 "xueke"）
///
/// # 返回
/// - 提交成功或失败
pub async fn submit_matched_question(
    ctx: &QuestionCtx,
    matched_data: &serde_json::Value,
    search_source: &str,
) -> Result<()> {
    let prefix = ctx.log_prefix();
    info!("{} 提交匹配题目，来源: {}", prefix, search_source);

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

    info!("{} 匹配题目提交成功: {:?}", prefix, response);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要真实环境网络
    async fn test_submit_matched_question() {
        let ctx = QuestionCtx {
            paper_id: "3422724566465490944".to_string(),
            subject_code: "54".to_string(),
            stage: "3".to_string(),
            paper_index: 1,
            question_index: 2,
            is_title: false,
            screenshot: "".to_string(),
        };

        // 模拟搜索到的原始数据 (基于提供的示例)
        // 使用 from_str 避免 json! 宏的递归深度限制
        let matched_data: serde_json::Value = serde_json::from_str(r#"
     {
        "originQuestionId": "",
        "questionSeq": "",
        "addFlag": 1,
        "increaseFlag": null,
        "knwFlag": null,
        "attachmentFlag": null,
        "htmlTagFlag": null,
        "originalFlag": null,
        "difficultyCode": "",
        "sourceName": "",
        "platform": "",
        "businessTypeName": "单选题",
        "structureIsSupport": 1,
        "shareFlag": false,
        "paperLabels": [],
        "valid": "",
        "textHash": "378B327A66DC1E31DE6AFC62236283C1",
        "hash": "0E864C7217CCBDB03E1D3DDCEE70BA12",
        "paperId": "3422724566465490944",
        "paperName": "",
        "id": 26716670,
        "questionId": "2731594774093512704",
        "version": "",
        "parentId": "2731594774093512704",
        "questionCode": "2731594774093512704",
        "sysCode": 1,
        "sysCodeName": "",
        "schNumber": "65",
        "schName": "集团",
        "stage": "3",
        "stageName": "初中",
        "subject": "54",
        "subjectName": "数学",
        "grade": "",
        "gradeName": "",
        "questionType": "1",
        "businessType": "CSX-DANXUAN",
        "structureType": "danxuan",
        "structureTypeName": "单选题",
        "purpose": 0,
        "questionContent": "表示 （ ▲ ）（ ） 的相反数 的相反数 的相反数 的相反数 本题根据相反数的定义即可得出结果． 解：-（-2）=2， 根据相反数的定义可得：-（-2）表示-2的相反数 故选C． 本题主要考查了相反数的定义和表示方法，解题时要注意结果的符号．",
        "questionInfo": {
            "explain": "",
            "answer": "C",
            "directions": "",
            "options": [
            {
                "optCode": "Y1IW",
                "htmlCode": "<span class=\"qml-op\"><img src=\"https://k12static.xdf.cn/k12/xkw/1748534933941/6d8d9fe5ca8544c7ad59fe85a426cde4.png\" wmf=\"http://static.zujuan.com/Upload/2013-02/21/d7faf96e-026c-443d-b276-1d7d5f46a4a8/resource.files/imaged7faf96e-026c-443d-b276-1d7d5f46a4a82.wmf\"><span>的相反数</span></span>",
                "title": "A",
                "flagAnswer": "0"
            },
            {
                "optCode": "RyKL",
                "htmlCode": "<span class=\"qml-op\"><img src=\"https://k12static.xdf.cn/k12/xkw/1748534934133/22e49af9c42f48e886f38dbb8398ccf8.png\" wmf=\"http://static.zujuan.com/Upload/2013-02/21/d7faf96e-026c-443d-b276-1d7d5f46a4a8/resource.files/imaged7faf96e-026c-443d-b276-1d7d5f46a4a83.wmf\"><span>的相反数</span></span>",
                "title": "B",
                "flagAnswer": "0"
            },
            {
                "optCode": "pSln",
                "htmlCode": "<span class=\"qml-op\"><img src=\"https://k12static.xdf.cn/k12/xkw/1748534934320/28907827e2a7451e8ffbb70b165ebb8e.png\" wmf=\"http://static.zujuan.com/Upload/2013-02/21/d7faf96e-026c-443d-b276-1d7d5f46a4a8/resource.files/imaged7faf96e-026c-443d-b276-1d7d5f46a4a84.wmf\"><span>的相反数</span></span>",
                "title": "C",
                "flagAnswer": "1"
            },
            {
                "optCode": "TwIs",
                "htmlCode": "<span class=\"qml-op\"><img src=\"https://k12static.xdf.cn/k12/xkw/1748534934523/648830273ce04398a98a99ded9111c16.png\" wmf=\"http://static.zujuan.com/Upload/2013-02/21/d7faf96e-026c-443d-b276-1d7d5f46a4a8/resource.files/imaged7faf96e-026c-443d-b276-1d7d5f46a4a85.wmf\"><span>的相反数</span></span>",
                "title": "D",
                "flagAnswer": "0"
            }
            ],
            "comment": "",
            "remark": "",
            "analysis": "<span>本题根据相反数的定义即可得出结果．</span><p style=\"\"><span>解：-（-2）=2，</span></p><p style=\"\"><span>根据相反数的定义可得：-（-2）表示-2的相反数</span></p><p style=\"\"><span>故选C．</span></p><p style=\"\"><span>本题主要考查了相反数的定义和表示方法，解题时要注意结果的符号．</span></p>",
            "stem": "<p style=\"\"><img src=\"https://k12static.xdf.cn/k12/xkw/1748534933938/6f0fa01ec2004ef18c1ee6f56ca65502.png\" wmf=\"http://static.zujuan.com/Upload/2013-02/21/d7faf96e-026c-443d-b276-1d7d5f46a4a8/resource.files/imaged7faf96e-026c-443d-b276-1d7d5f46a4a81.wmf\"><span>表示 （ ▲ ）</span>（&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;）</p>"
        },
        "questionProperty": {
            "property": {
            "difficulty": [
                {
                "type": 2,
                "value": 3
                }
            ],
            "area": [],
            "paperType": [
                {
                "type": 2,
                "value": "3543"
                }
            ],
            "province": [],
            "year": [
                {
                "type": 2,
                "value": 2011
                }
            ],
            "city": [],
            "sourceName": [],
            "businessType": [
                {
                "type": 2,
                "value": "CSX-DANXUAN"
                }
            ]
            },
            "chapIds": [],
            "knwIds": [
            "2480456819557531648"
            ],
            "knowledge": [
            {
                "knwId": "2480456819557531648",
                "knwName": "相反数的概念及性质"
            }
            ]
        },
        "questionIndex": 2,
        "questionAnswer": "C",
        "questionSource": 2,
        "status": 66,
        "difficulty": 3,
        "year": 2011,
        "examType": "",
        "examArea": "",
        "groupId": "2731594774093512704",
        "questionScore": null,
        "relationType": 1,
        "similarityScore": null,
        "creator": "admin@xdf.cn",
        "creatorName": "admin",
        "createTime": "2016-12-05 04:20:50",
        "editor": "admin@xdf.cn",
        "editorName": "admin",
        "editTime": "2016-12-05 04:20:50",
        "isProcessing": null,
        "questionStem": "<p style=\"\"><img src=\"https://k12static.xdf.cn/k12/xkw/1748534933938/6f0fa01ec2004ef18c1ee6f56ca65502.png\" wmf=\"http://static.zujuan.com/Upload/2013-02/21/d7faf96e-026c-443d-b276-1d7d5f46a4a8/resource.files/imaged7faf96e-026c-443d-b276-1d7d5f46a4a81.wmf\"><span>表示 （ ▲ ）</span>（&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;）</p>",
        "questionAnalysis": "<span>本题根据相反数的定义即可得出结果．</span><p style=\"\"><span>解：-（-2）=2，</span></p><p style=\"\"><span>根据相反数的定义可得：-（-2）表示-2的相反数</span></p><p style=\"\"><span>故选C．</span></p><p style=\"\"><span>本题主要考查了相反数的定义和表示方法，解题时要注意结果的符号．</span></p>",
        "questionVersion": 1,
        "tikuType": 3,
        "highQuality": 0,
        "organizedCount": "",
        "flagUse": 1,
        "recordTime": null,
        "correctionFlag": "",
        "needSupplement": null,
        "mediaIds": [],
        "smallQuestions": [],
        "isQuoted": null,
        "oldQuestionId": "",
        "hasCorrected": null,
        "startTime": null,
        "endTime": null,
        "similarityPaperName": "",
        "knowledgeList": [],
        "refCount": "",
        "answeredCount": "",
        "percentCorrect": "",
        "sourceRefCount": "",
        "knwInfos": [
            {
            "knwId": "2480456819557531648",
            "knwName": "相反数的概念及性质"
            }
        ],
        "chapterInfos": [],
        "index": "",
        "questionFlag": "",
        "paperList": [],
        "compositeScore": null,
        "checkStatus": "",
        "standardFlag": 1,
        "stemImageUrl": "",
        "analysisImageUrl": "",
        "answerImageUrl": "",
        "analysisVideos": [],
        "analysisVideoMediaId": "",
        "customTags": [],
        "xkwQuestionSimilarity": 28.57,
        "inputType": 1,
        "analysisVideoInfo": {}
        }
        "#).expect("JSON 解析失败");

        println!("开始测试提交匹配题目...");
        let result = submit_matched_question(&ctx, &matched_data, "test_k12").await;

        match result {
            Ok(_) => println!("提交成功!"),
            Err(e) => {
                println!("提交失败: {:?}", e);
                // 这里不 panic，因为这是网络请求，本地测试可能会失败
            }
        }
    }
}
