//! LLM 服务核心实现

use anyhow::Result;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPartImage,
        ChatCompletionRequestMessageContentPartText, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
        ChatCompletionRequestUserMessageContentPart, CreateChatCompletionRequestArgs, FinishReason,
        ImageDetail, ImageUrl,
    },
};

use tracing::{debug, error, warn};

use crate::{api::{llm::message_builder::build_send_messages, search::SearchResult}, config::get};
use crate::config::AppConfig;

/// LLM 服务
///
/// 职责：
/// - 调用 LLM API 进行题目匹配判断
/// - 提供通用的 LLM 调用接口
/// - 只处理单个题目的匹配
/// - 不出现 Vec<Question>
/// - 不出现 paper_id / question_index
/// - 不关心流程顺序
pub struct LlmService {
    pub(crate) client: Client<OpenAIConfig>,
    pub(crate) model_name: String,
}

impl LlmService {
    /// 创建新的 LLM 服务
    pub fn new(config: &AppConfig) -> Self {
        // 配置 OpenAI 客户端（兼容 OpenAI API 的服务）
        let openai_config = OpenAIConfig::new()
            .with_api_key(&config.llm_api_key)
            .with_api_base(&config.llm_api_base_url);

        let client = Client::with_config(openai_config);

        Self {
            client,
            model_name: config.llm_model_name.clone(),
        }
    }

    /// 发送消息到 LLM
    pub async fn send_to_llm(
        &self,
        user_message: &str,
        system_message: Option<&str>,
        imgs: Option<&[String]>,
    ) -> Result<String> {
        debug!("调用 LLM API，模型: {}", self.model_name);
        debug!("用户消息长度: {} 字符", user_message.len());
        if let Some(img_urls) = imgs {
            debug!("包含 {} 张图片", img_urls.len());
        }

        // 构建消息列表
        let mut messages = Vec::new();

        // 添加系统消息（如果提供）
        if let Some(sys_msg) = system_message {
            let system_msg = ChatCompletionRequestSystemMessageArgs::default()
                .content(sys_msg)
                .build()?;
            messages.push(ChatCompletionRequestMessage::System(system_msg));
        }

        // 构建用户消息内容（支持图片）
        let user_msg = if let Some(img_urls) = imgs {
            if !img_urls.is_empty() {
                let mut content_parts: Vec<ChatCompletionRequestUserMessageContentPart> =
                    Vec::new();

                // 添加文本部分
                content_parts.push(ChatCompletionRequestUserMessageContentPart::Text(
                    ChatCompletionRequestMessageContentPartText {
                        text: user_message.to_string(),
                    },
                ));

                // 添加图片部分
                for url in img_urls.iter() {
                    content_parts.push(ChatCompletionRequestUserMessageContentPart::ImageUrl(
                        ChatCompletionRequestMessageContentPartImage {
                            image_url: ImageUrl {
                                url: url.clone(),
                                detail: Some(ImageDetail::Auto), // Auto, High, Low
                            },
                        },
                    ));
                }

                debug!("使用 Vision API，包含 {} 张图片", img_urls.len());

                // 构建包含多部分内容的用户消息
                ChatCompletionRequestUserMessageArgs::default()
                    .content(ChatCompletionRequestUserMessageContent::Array(
                        content_parts,
                    ))
                    .build()?
            } else {
                // 没有图片，只有文本
                ChatCompletionRequestUserMessageArgs::default()
                    .content(user_message)
                    .build()?
            }
        } else {
            // 没有图片参数，只有文本
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_message)
                .build()?
        };

        messages.push(ChatCompletionRequestMessage::User(user_msg));

        debug!("message:{}", serde_json::to_string(&messages).unwrap());

        // 构建请求
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .messages(messages)
            .temperature(0.3)
            // .max_tokens(1000u32)
            .build()?;

        // 调用 API
        let response = self.client.chat().create(request).await.map_err(|e| {
            warn!("LLM API 网络/协议层错误: {}", e);
            anyhow::anyhow!("LLM API 调用失败: {}", e)
        })?;

        debug!("LLM API 调用成功");

        if let Some(choice) = response.choices.first() {
            debug!("LLM Finish Reason: {:?}", choice.finish_reason);
            if let Some(reason) = &choice.finish_reason {
                if matches!(reason, FinishReason::ContentFilter) {
                    return Err(anyhow::anyhow!("请求被 AI 内容风控拦截"));
                }
            }
        }

        // 提取响应内容
        let raw_content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_default();

        if raw_content.trim().is_empty() {
            warn!(
                "LLM 返回内容为空! Raw length: {}, Bytes: {:?}",
                raw_content.len(),
                raw_content.as_bytes()
            );
            return Err(anyhow::anyhow!("LLM 返回了空字符串/纯空白"));
        }

        Ok(raw_content.trim().to_string())
    }

    /// 解析 LLM 的响应
    /// 预期响应： "0", "1", ... 或 "None"
    pub(crate) fn parse_match_response(
        &self,
        response: &str,
        candidates_count: usize,
    ) -> Result<Option<usize>> {
        let trimmed = response.trim();
        if trimmed.is_empty() {
            error!("LLM 返回内容为空");
            return Err(anyhow::anyhow!("LLM 返回内容为空"));
        }

        // 1. 检查 "None"
        if trimmed.eq_ignore_ascii_case("None") {
            return Ok(None);
        }

        // 2. 尝试解析为数字
        if let Ok(idx) = trimmed.parse::<usize>() {
            if idx < candidates_count {
                return Ok(Some(idx));
            } else {
                anyhow::bail!("LLM 返回的索引 {} 超出范围 (0..{})", idx, candidates_count);
            }
        }

        // 3. 既不是 None 也不是纯数字
        anyhow::bail!("LLM 返回格式非法，内容如下: {}", response);
    }

    /// 查找最佳匹配的题目
    pub async fn find_best_match_index(
        &self,
        search_results: &[SearchResult],
        target_screenshot: &str,
    ) -> Result<Option<usize>> {
        if search_results.is_empty() {
            anyhow::bail!("搜索结果为空，无法进行匹配");
        }

        let (user_message, system_message, all_images) =
            build_send_messages(search_results, target_screenshot).await;

        let imgs_slice = if all_images.is_empty() {
            None
        } else {
            Some(all_images.as_slice())
        };

        // --- 开始重试循环 (最多 3 次) ---
        let max_retries = 3;

        for attempt in 1..=max_retries {
            debug!("LLM 匹配尝试第 {}/{} 次", attempt, max_retries);

            // 1. 发送请求
            let response_result = self
                .send_to_llm(&user_message, Some(&system_message), imgs_slice)
                .await;

            // 2. 处理结果
            match response_result {
                Ok(response) => {
                    // 只要能正常解析出 Some(i) 或 None，就算成功
                    match self.parse_match_response(&response, search_results.len()) {
                        Ok(result) => {
                            if let Some(idx) = result {
                                debug!("LLM 成功匹配到索引: {}", idx);
                            } else {
                                debug!("LLM 认为没有匹配项 (None)");
                            }
                            return Ok(result);
                        }
                        Err(e) => {
                            warn!(
                                "解析失败 (第 {} 次): {}, 响应内容: {}",
                                attempt, e, response
                            );
                            if attempt == max_retries {
                                warn!("重试耗尽，无法解析响应，默认返回 None");
                                return Ok(None);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("LLM 调用失败 (第 {} 次): {}", attempt, e);
                    if e.to_string().contains("内容风控") {
                        warn!("触发风控，停止重试");
                        return Ok(None);
                    }
                    if attempt == max_retries {
                        warn!("重试耗尽，LLM 无法给出有效响应");
                        return Ok(None);
                    }
                }
            }
        }

        Ok(None)
    }
}


impl Default for LlmService {
    fn default() -> Self {
        LlmService::new(get())
    }
}