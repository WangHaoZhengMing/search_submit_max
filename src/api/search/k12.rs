use anyhow::Result;
use serde_json::json;
use tracing::info;

use crate::api::send_request::send_api_request;
use super::SearchResult;

/// K12 题库搜索
///
/// # 参数
/// * `stage` - 学段，如 "3" 表示高中
/// * `subject` - 学科，如 "61" 表示物理
/// * `text` - 搜索文本
///
/// # 返回
/// 返回搜索结果列表
pub async fn k12_search(stage: &str, subject: &str, text: &str) -> Result<Vec<SearchResult>> {
    let url = "https://tps-tiku-api.staff.xdf.cn/api/questionsimilar/queryByText";

    let payload = json!({
        "text": text,
        "subject": subject,
        "stage": stage,
    });

    const MAX_RETRIES: usize = 3;
    let mut last_result = None;

    for attempt in 1..=MAX_RETRIES {
        info!(
            "发送 K12 题库搜索请求 (尝试 {}/{}): stage={}, subject={}, text={}",
            attempt, MAX_RETRIES, stage, subject, text
        );

        let result = send_api_request(url, &payload).await?;

        // 检查 data 字段是否存在且不为空
        if let Some(data) = result.get("data") {
            if let Some(arr) = data.as_array() {
                if !arr.is_empty() {
                    info!("K12 题库搜索完成，找到 {} 条结果", arr.len());
                    // 解析为 SearchResult 列表
                    let search_results: Vec<SearchResult> = serde_json::from_value(data.clone())?;
                    return Ok(search_results);
                }
            }
        }

        info!(
            "K12 题库搜索返回空结果或缺少 data 字段 (尝试 {}/{})",
            attempt, MAX_RETRIES
        );
        last_result = Some(result);

        if attempt < MAX_RETRIES {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    Err(anyhow::anyhow!(
        "K12 题库搜索失败: 重试 {} 次后仍未获取到有效数据。最后响应: {}",
        MAX_RETRIES,
        serde_json::to_string(&last_result).unwrap_or_default()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::logger;

    #[tokio::test]
    async fn test_k12_search() {
        logger::init_test();

        let result = k12_search("3", "61", "你好").await;

        assert!(result.is_ok());
        let response = result.unwrap();
        println!(
            "搜索结果: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );
    }
}