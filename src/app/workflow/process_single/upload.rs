use anyhow::{Context, Result};
use std::io::Write;
use tempfile::NamedTempFile;
use tracing::info;

use crate::api::base64_to_img::base64_to_png_img;
use crate::api::upload::img::{upload_image_haoranwang, upload_img};
use crate::app::workflow::QuestionCtx;

/// 上传截图到 COS
///
/// # 流程
/// 1. 将 Base64 截图解码为图片字节
/// 2. 保存到临时文件
/// 3. 上传到 COS
/// 4. 返回 CDN URL
pub async fn upload_screenshot(ctx: &QuestionCtx) -> Result<String> {
    let prefix = ctx.log_prefix();
    
    info!("{} 开始上传截图", prefix);

    // 1. Base64 解码
    let img_bytes = base64_to_png_img(&ctx.screenshot)
        .map_err(|e| anyhow::anyhow!("{} Base64 解码失败: {}", prefix, e))?;

    info!("{} Base64 解码成功，大小: {} 字节", prefix, img_bytes.len());

    // 2. 写入临时文件
    let mut temp_file = NamedTempFile::new()
        .with_context(|| format!("{} 创建临时文件失败", prefix))?;
    
    temp_file
        .write_all(&img_bytes)
        .with_context(|| format!("{} 写入临时文件失败", prefix))?;

    let temp_path = temp_file.path().to_str()
        .ok_or_else(|| anyhow::anyhow!("{} 临时文件路径无效", prefix))?
        .to_string();

    info!("{} 临时文件: {}", prefix, temp_path);

    // 3. 上传到 COS
    let cdn_url = upload_img(&temp_path)
        .await
        .with_context(|| format!("{} 上传到 COS 失败", prefix))?;

    info!("{} 截图上传成功: {}", prefix, cdn_url);

    Ok(cdn_url)
}


pub async fn upload_screenshot_tohaoran(ctx: &QuestionCtx) -> Result<String> {
    let prefix = ctx.log_prefix();
    
    info!("{} 开始上传截图", prefix);

    // 1. Base64 解码
    let img_bytes = base64_to_png_img(&ctx.screenshot)
        .map_err(|e| anyhow::anyhow!("{} Base64 解码失败: {}", prefix, e))?;

    info!("{} Base64 解码成功，大小: {} 字节", prefix, img_bytes.len());

    // 2. 写入临时文件
    let mut temp_file = NamedTempFile::new()
        .with_context(|| format!("{} 创建临时文件失败", prefix))?;
    
    temp_file
        .write_all(&img_bytes)
        .with_context(|| format!("{} 写入临时文件失败", prefix))?;

    let temp_path = temp_file.path().to_str()
        .ok_or_else(|| anyhow::anyhow!("{} 临时文件路径无效", prefix))?
        .to_string();

    info!("{} 临时文件: {}", prefix, temp_path);

    // 3. 上传到 COS
    let cdn_url = upload_image_haoranwang(&temp_path)
        .await
        .with_context(|| format!("{} 上传到 COS 失败", prefix))?;

    info!("{} 截图上传成功: {}", prefix, cdn_url);

    Ok(cdn_url)
}