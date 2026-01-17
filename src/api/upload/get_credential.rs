use crate::api::send_request::send_api_request;
use anyhow::Result;
use serde_json::Value;
use tracing::{debug, info};

/// 获取腾讯云 COS 上传凭证（图片和 PDF 通用）
pub async fn get_credential() -> Result<Value> {
    let play_load = serde_json::json!({
      "storageType": "cos",
      "securityLevel": 1
    });
    info!("获取上传凭证...");
    let result = send_api_request(
        "https://tps-tiku-api.staff.xdf.cn/attachment/get/credential",
        &play_load,
    )
    .await?;
    
    // 记录完整的响应用于调试
    debug!("凭证响应: {}", serde_json::to_string_pretty(&result).unwrap_or_default());
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::logger;

    #[tokio::test]
    async fn test_get_credential() {
        logger::init_test();
        let result = get_credential().await;
        assert!(result.is_ok());
    }
}
