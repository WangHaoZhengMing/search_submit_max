use std::result::Result;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::llm::service::LlmService;
use crate::app::workflow::QuestionCtx;
use crate::app::workflow::process_single::result::StepError;

#[derive(Debug, Deserialize, Serialize)]
pub struct SingleQuestion {
    pub stem: String,
    pub options: Vec<String>,
    pub answer: usize, // 从0开始
    pub analysis: String,
}

impl SingleQuestion {
    pub fn to_payload(&self, ctx: &QuestionCtx) -> Value {
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Option {
            title: String,
            html_code: String,
            opt_code: String,
            flag_answer: String,
        }
        let mut options: Vec<Option> = vec![];
        let mut title = String::new();
        for (i, opt_text) in self.options.iter().enumerate() {
            title = match i {
                0 => "A",
                1 => "B",
                2 => "C",
                3 => "D",
                4 => "E",
                5 => "F",
                6 => "G",
                7 => "H",
                _ => "I",
            }
            .to_string();
            let opt_code = format!("{:04x}", 0xAB00 + i);
            let flag_answer = if i == self.answer {
                "1".to_string()
            } else {
                "0".to_string()
            };
            options.push(Option {
                title: title,
                html_code: format!("<p>{}</p>\n", opt_text),
                opt_code,
                flag_answer,
            });
        }

        // 逻辑：找到 flag_answer 为 "1" 的那个选项的索引 (index)，然后把 index 转成 A/B/C
        let answer_str: String = options
            .iter()
            .enumerate() // 获取索引: (0, item), (1, item)...
            .filter(|(_, opt)| opt.flag_answer == "1") // 筛选出正确选项
            .map(|(index, _)| ((b'A' + index as u8) as char).to_string()) // 0->A, 1->B
            .collect::<Vec<String>>() // 收集结果
            .join(""); // 拼接（如果是多选题，结果可能是 "AB"）

        json!(
            {
            "structureType": "danxuan",
            "businessType": "CSX-DANXUAN",
            "questionInfo": {
                "stem": format!("<p>{}</p>\n", self.stem),
                "options": options,
                "answer": answer_str,
                "analysis": format!("<p>{}</p>\n", self.analysis)
            },
            "questionSource": 2,
            "questionType": 1,
            "relationType": 0,
            "sysCode": 1,
            "paperId": ctx.paper_id,//to be filled
            "questionIndex": ctx.question_index,// to be filled
            "inputType": 1,
            "addFlag": 1
            }
        )
    }
}

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
    let system_message = r#"你是一个专业的试题数字化助手。请分析用户提供的题目图片，提取题目信息。
        
        任务：识别题型并生成 TOML 格式数据。不要输出 [[questions]]，直接输出字段
        
        要求：
        1. 只支持“选择题”和主观题目。其他题型（填空题目）返回 "NotSupport"。
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
		8. 注意换行。使用html的换行语法。

        这个是选择题目的输出结构示例：
		type: "CSX-DANXUAN"
        stem = '''
        $\frac{1}{2025}$ 的相反数是（　　）。这里面也可以放 html。比如表格，html描述的图形（不是svg，这里不支持svg） 等复杂结构。但是遇到特殊字符不要直接写unicode。要用html的符号写法。
        '''
        options = [
            '''$2025$''', 
            '''$-2025$''', 
            '''$\frac{1}{2025}$''', 
            '''$-\frac{1}{2025}$'''
        ]  //每个选项这里面也可以放 html。比如表格，html描述的图形（不是svg，这里不支持svg） 等复杂结构。但是遇到特殊字符不要直接写unicode。要用html的符号写法。如果遇到需要画图的再画。
        answer = 3 // 注意：答案是从0开始计数的索引
        analysis = '''
        本题考查了相反数的定义。
        解：互为相反数。
        故选：D．
        '''    //格式：考点+分析+故答案为：



		//这个是主观题目的例子：
		type: "CSX-JIEDA"
		stem = '''
        计算：
        '''
        answer = '''
        解：原式 
        ''' 
        analysis = '''
        本题考查了实数的运算。

        '''
        "#;

    let user_message = "按照我的截图生成 toml 格式生成题目，如果不是选择题或主观题，请仅返回 NotSupport 字符串。其它的话不要ｔｍｄ多说";

    let imgs = vec![screenshot_url.to_string()];

    let llm_response = llm
        .send_to_llm(user_message, Some(system_message), Some(imgs.as_slice()))
        .await?;

    if llm_response.contains("NotSupport") {
        return Err(StepError::UnsupportedQuestion);
    }
    // 🟢【新增】清洗 Markdown 代码块标记
    let clean_response = llm_response
        .trim()
        .trim_start_matches("```toml")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim(); // 再次 trim 去除首尾可能留下的换行符
    // println!("Cleaned LLM Response:\n{}", clean_response);
    if clean_response.contains("CSX-DANXUAN") {
        // 使用清洗后的 clean_response 进行解析
        let temp_ques = toml::from_str::<SingleQuestion>(clean_response).map_err(|e| {
            StepError::LlmBuildFailed(format!(
                "can not parser toml: {}\nRaw: {}",
                e, clean_response
            ))
        })?;

        let payload = temp_ques.to_payload(&ctx);
        Ok(payload)
    } else {
        let temp_ques = toml::from_str::<ZhuguanQuestion>(clean_response).map_err(|e| {
            StepError::LlmBuildFailed(format!(
                "can not parser toml: {}\nRaw: {}",
                e, clean_response
            ))
        })?;

        let payload = temp_ques.to_payload(&ctx);
        Ok(payload)
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_question_via_llm() {
        let ctx = QuestionCtx {
            paper_id: "3425387499337388032".to_string(),
            subject_code: "54".to_string(),
            stage: "3".to_string(),
            paper_index: 1,
            question_index: 1,
            is_title: false,
            screenshot: "".to_string(),
            not_include_title_index: 0,
        };

        let screenshot_url = "https://tiku-1252350207.cos.ap-beijing.myqcloud.com/k12-paperxdfUploadtikuImageDir/6-1768805217642.png";
        let ocr_text = "已知集合A={1,2,3},集合B={2,3,4},则A与B的交集是()";

        match build_question_via_llm(&ctx, ocr_text, screenshot_url).await {
            Ok(payload) => {
                println!("构建成功，Payload: {}", payload);
            }
            Err(e) => {
                println!("构建失败，错误: {:?}", e);
            }
        }
    }
}
