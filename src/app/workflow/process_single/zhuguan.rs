use std::result::Result;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::llm::service::LlmService;
use crate::app::workflow::QuestionCtx;
use crate::app::workflow::process_single::result::StepError;

#[derive(Debug, Deserialize, Serialize)]
pub struct ZhuguanQuestion {
    pub stem: String,
    pub answer: String, // 主观题答案通常是一段文本或HTML
    pub analysis: String,
}

impl ZhuguanQuestion {
    pub fn to_payload(&self, ctx: &QuestionCtx) -> Value {
        // 主观题 payload 中 options 通常为空数组
        let options: Vec<String> = vec![]; 
        
        json!(
            {
            "structureType": "zhuguan",
            "businessType": "CSX-JIEDA",
            "questionInfo": {
                // 主观题题干通常不需要像选择题那样加括号占位符，直接包裹 p 标签即可
                "stem": format!("<p>{}</p>\n", self.stem),
                "options": options,
                "answer": format!("<p>{}</p>\n", self.answer),
                "analysis": format!("<p>{}</p>\n", self.analysis)
            },
            // 主观题特有的属性，根据提供的 sample 似乎有一个 questionProperty，这里先给默认空
            "questionProperty": {
                "knwIds": []
            },
            "questionSource": 2,
            "questionType": 1,
            "relationType": 0,
            "sysCode": 1,
            "paperId": ctx.paper_id, // to be filled
            "questionIndex": ctx.question_index, // to be filled
            "inputType": 1,
            "addFlag": 1
            }
        )
    }
}

pub async fn build_question_via_llm(
    ctx: &QuestionCtx,
    _ocr_text: &str,
    screenshot_url: &str,
) -> Result<Value, StepError> {
    let llm = LlmService::default();
    
    // 针对主观题调整 Prompt
    let system_message = 
        r#"你是一个专业的试题数字化助手。请分析用户提供的题目图片，提取题目信息。
        
        任务：识别题型并生成 TOML 格式数据。不要输出 [[questions]]，直接输出字段
        
        要求：
        1. 针对“解答题”、“计算题”、“证明题”等主观题型提取。如果是选择题，返回 "NotSupport"。
        2. 必须输出标准的 TOML 格式。
        3. 【核心要求】所有字符串值必须使用三个单引号 ''' (字面量字符串) 包裹。
        4. 【核心要求】为了格式安全，''' 之后的内容必须换行书写，结尾的 ''' 也要独占一行。
           例如：
           stem = '''
           这里是内容
           '''
        5. 绝对不要对 LaTeX 公式中的 \ 进行转义，保持原样。
        6. 不要输出 ```toml，只输出内容。
        7. 所有的数学符号都要用 LaTeX 的\+名称书写，包括非常常见的><= etc.。因为防止在渲染时出现问题。不要直接使用 Unicode 数学符号。

        输出结构示例：

        stem = '''
        计算：:markdown-math{single="true" encoded="%5Csqrt%7B16%7D%20%2B%20%7C-3%7C%20-%20(%20%5Cpi%20-%203%20)%5E0"}。
        '''
        answer = '''
        解：原式 :markdown-math{single="true" encoded="%3D%204%20%2B%203%20-%201%20%3D%206"}。
        ''' 
        analysis = '''
        本题考查了实数的运算。
        :markdown-math{single="true" encoded="%5Csqrt%7B16%7D%3D4"}，
        :markdown-math{single="true" encoded="%7C-3%7C%3D3"}，
        :markdown-math{single="true" encoded="(%20%5Cpi%20-%203%20)%5E0%3D1"}。
        故答案为 :markdown-math{single="true" encoded="6"}。
        '''
        "#;

    let user_message = "按照我的截图生成 toml 格式的主观题（解答/计算/证明），如果是选择题，请仅返回 NotSupport 字符串。";

    let imgs = vec![screenshot_url.to_string()];

    let llm_response = llm
        .send_to_llm(user_message, Some(system_message), Some(imgs.as_slice()))
        .await?;

    if llm_response.contains("NotSupport") {
        return Err(StepError::UnsupportedQuestion);
    }
    
    // 清洗 Markdown 代码块标记
    let clean_response = llm_response
        .trim()
        .trim_start_matches("```toml")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
        
    println!("Cleaned LLM Response (Zhuguan):\n{}", clean_response);

    // 解析 TOML
    let temp_ques = toml::from_str::<ZhuguanQuestion>(clean_response).map_err(|e| {
        StepError::LlmBuildFailed(format!(
            "can not parser toml: {}\nRaw: {}",
            e, clean_response
        ))
    })?;

    let payload = temp_ques.to_payload(&ctx);

    Ok(payload)
}
