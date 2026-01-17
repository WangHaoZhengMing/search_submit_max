use anyhow::{Context, Result};
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use serde_json::Value;
use std::{fs::File, path::Path};
use std::io::Read;
use tracing::{error, info};

use crate::app::base64_decode::Base64Decode;

use super::get_credential::get_credential;
#[allow(dead_code)]
/// 从凭证 JSON 中提取上传信息
fn parse_credential_info(json_data: &Value) -> Result<CredentialInfo> {
    let data = json_data.get("data").context("缺少 data 字段")?;

    let creds = data.get("credentials").with_context(|| {
        error!("API 响应中缺少 credentials 字段，完整响应: {}", 
            serde_json::to_string_pretty(json_data).unwrap_or_default());
        "缺少 credentials 字段"
    })?;
    let tmp_secret_id = creds
        .get("tmpSecretId")
        .and_then(|v| v.as_str())
        .context("缺少 tmpSecretId")?;
    let tmp_secret_key = creds
        .get("tmpSecretKey")
        .and_then(|v| v.as_str())
        .context("缺少 tmpSecretKey")?;
    let session_token = creds
        .get("sessionToken")
        .and_then(|v| v.as_str())
        .context("缺少 sessionToken")?;

    let bucket = data
        .get("bucket")
        .and_then(|v| v.as_str())
        .context("缺少 bucket")?;
    let region = data
        .get("region")
        .and_then(|v| v.as_str())
        .context("缺少 region")?;
    let key_prefix = data
        .get("keyPrefix")
        .and_then(|v| v.as_str())
        .context("缺少 keyPrefix")?;
    let cdn_domain = data
        .get("cdnDomain")
        .and_then(|v| v.as_str())
        .context("缺少 cdnDomain")?;

    Ok(CredentialInfo {
        tmp_secret_id: tmp_secret_id.to_string(),
        tmp_secret_key: tmp_secret_key.to_string(),
        session_token: session_token.to_string(),
        bucket: bucket.to_string(),
        region: region.to_string(),
        key_prefix: key_prefix.to_string(),
        cdn_domain: cdn_domain.to_string(),
    })
}

#[allow(dead_code)]
#[derive(Debug)]
struct CredentialInfo {
    tmp_secret_id: String,
    tmp_secret_key: String,
    session_token: String,
    bucket: String,
    region: String,
    key_prefix: String,
    cdn_domain: String,
}
#[allow(dead_code)]
/// 上传图片到腾讯云 COS（内部函数，使用已有凭证）
async fn upload_image_to_cos_with_credential(
    credential_json: &Value,
    local_file_path: &str,
) -> Result<String> {
    info!("开始上传图片: {}", local_file_path);

    // 1. 解析凭证信息
    let cred_info = parse_credential_info(credential_json)?;
    info!(
        "凭证信息解析成功，Bucket: {}, Region: {}",
        cred_info.bucket, cred_info.region
    );

    // 2. 创建 S3 凭证对象
    let credentials = Credentials::new(
        Some(&cred_info.tmp_secret_id),
        Some(&cred_info.tmp_secret_key),
        Some(&cred_info.session_token.clone()),
        None,
        None,
    )?;

    // 3. 配置腾讯云 COS 区域
    let region = Region::Custom {
        region: cred_info.region.clone(),
        endpoint: format!("https://cos.{}.myqcloud.com", cred_info.region),
    };

    // 4. 初始化 Bucket
    let bucket = Bucket::new(&cred_info.bucket, region, credentials)?;

    // 5. 读取本地文件
    let mut file =
        File::open(local_file_path).context(format!("无法打开文件: {}", local_file_path))?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .context("读取文件内容失败")?;

    // 6. 生成唯一的云端文件名
    let extension = std::path::Path::new(local_file_path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("png");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let object_key = format!(
        "{}/{}-{}.{}",
        cred_info.key_prefix,
        timestamp,
        rand::random::<u32>(),
        extension
    );

    info!("上传路径: {}", object_key);

    // 7. 执行上传
    let response = bucket.put_object(&object_key, &contents).await?;

    if response.status_code() == 200 {
        // 8. 拼接最终的 CDN URL
        let final_url = format!("https://{}/{}", cred_info.cdn_domain, object_key);
        info!("图片上传成功！最终 URL: {}", final_url);
        Ok(final_url)
    } else {
        Err(anyhow::anyhow!(
            "上传失败，状态码: {}",
            response.status_code()
        ))
    }
}



/// 上传图片到浩然网的 COS (tiku-1396614861)
/// 
/// # 参数
/// * `local_file_path` - 本地图片文件的路径
/// * `secret_id` - 腾讯云 SecretId (建议从环境变量读取)
/// * `secret_key` - 腾讯云 SecretKey (建议从环境变量读取)
pub async fn upload_image_haoranwang(
    local_file_path: &str,
) -> Result<String> {
    info!("开始上传图片流程: {}", local_file_path);

    // --- 配置信息 ---
    let bucket_name = "tiku-1396614861";
    let region_name = "ap-beijing";
    // 你的 COS 访问域名 (通常是 bucket名.cos.region.myqcloud.com)
    // 如果你有自定义 CDN 域名，可以在这里替换，例如 "https://cdn.haoranwang.com"
    let base_url = format!("https://{}.cos.{}.myqcloud.com", bucket_name, region_name);
    
    let secret_id = "QUtJRDVWa25zdlo2WWJXSHNUek9lamJIbTRDOHRTbnVzaUxr".parse_as_base64()?.to_string();
    let secret_key = "SE5HOXllN1p6R1BSTjZmNUpRTks2aUJXbXNybFR5R00=".parse_as_base64()?.to_string();

    
    // --- 1. 创建凭证 ---
    // 使用永久密钥，不需要 session_token
    let credentials = Credentials::new(
        Some(&secret_id),
        Some(&secret_key),
        None, 
        None, 
        None
    )?;

    // --- 2. 配置区域 ---
    let region = Region::Custom {
        region: region_name.to_string(),
        endpoint: format!("https://cos.{}.myqcloud.com", region_name),
    };

    // --- 3. 初始化 Bucket ---
    let bucket = Bucket::new(bucket_name, region, credentials)?;

    // --- 4. 读取本地文件 ---
    let path = Path::new(local_file_path);
    let mut file = File::open(path).context(format!("无法打开文件: {}", local_file_path))?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).context("读取文件内容失败")?;

    // --- 5. 生成云端文件名 (Key) ---
    // 获取扩展名，默认为 png
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("png");

    // 生成唯一文件名: images/时间戳-随机数.后缀
    // 建议加上文件夹前缀(如 images/)，方便管理
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    let object_key = format!(
        "images/{}-{}.{}", 
        timestamp, 
        rand::random::<u32>(), 
        extension
    );

    info!("目标云端路径: {}", object_key);

    // --- 6. 执行上传 ---
    // content_type 设为 "application/octet-stream" 或者根据扩展名自动判断
    // rust-s3 的 put_object 会自动处理基本的 content-type
    let response = bucket.put_object(&object_key, &contents).await?;

    // --- 7. 处理结果 ---
    if response.status_code() == 200 {
        let final_url = format!("{}/{}", base_url, object_key);
        info!("图片上传成功！URL: {}", final_url);
        Ok(final_url)
    } else {
        let err_msg = format!("上传失败，状态码: {}", response.status_code());
        error!("{}", err_msg);
        Err(anyhow::anyhow!(err_msg))
    }
}

#[allow(dead_code)]
/// 上传图片的完整流程：获取凭证 -> 上传图片 -> 返回 URL
pub async fn upload_img(local_file_path: &str) -> Result<String> {
    info!("开始上传图片流程: {}", local_file_path);

    // 1. 获取上传凭证
    let credential = get_credential().await?;

    // 2. 上传图片到腾讯云 COS
    let image_url = upload_image_to_cos_with_credential(&credential, local_file_path).await?;

    info!("图片上传完成，URL: {}", image_url);
    Ok(image_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::logger;

    #[tokio::test]
    async fn test_upload_image() {
        logger::init_test();

        // 获取凭证并测试解析
        let credential = get_credential().await.expect("获取凭证失败");
        let parsed = parse_credential_info(&credential);
        assert!(parsed.is_ok());
        println!("凭证信息: {:?}", parsed.unwrap());
    }
}
