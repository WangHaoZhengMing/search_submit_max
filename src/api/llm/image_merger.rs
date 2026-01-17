//! 图片合并处理模块

use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, GenericImage, ImageFormat};
use std::io::Cursor;

/// 下载单张图片
async fn download_image(
    client: &reqwest::Client,
    url: &str,
) -> Result<DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client.get(url).send().await?;
    
    if !resp.status().is_success() {
        return Err(format!("HTTP 错误: {}", resp.status()).into());
    }
    
    let bytes = resp.bytes().await?;
    let img = image::load_from_memory(&bytes)?;
    Ok(img)
}


pub async fn smart_merge_images(
    urls: &[String],
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    if urls.is_empty() {
        return Ok(Vec::new());
    }

    // 1. 并发下载所有图片 (带超时和重试机制)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut tasks = Vec::new();
    for url in urls {
        let client = client.clone();
        let url = url.clone();
        
        let task = tokio::spawn(async move {
            const MAX_RETRIES: usize = 3;
            let mut last_error = None;
            
            for attempt in 1..=MAX_RETRIES {
                match download_image(&client, &url).await {
                    Ok(img) => return Ok(img),
                    Err(e) => {
                        last_error = Some(e);
                        if attempt < MAX_RETRIES {
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            }
            
            Err(last_error.unwrap_or_else(|| {
                format!("下载图片失败: {}", url).into()
            }))
        });
        tasks.push(task);
    }

    let mut loaded_images = Vec::new();
    for task in tasks {
        let img = task.await??;
        loaded_images.push(img);
    }

    // 2. 分组逻辑 (Chunking)
    // 如果一张图加进去超过 2000px，就切分到下一组
    let max_height_limit = 2000;
    let padding = 20; // 图片之间的间隔
    let mut groups: Vec<Vec<DynamicImage>> = Vec::new();
    let mut current_group: Vec<DynamicImage> = Vec::new();
    let mut current_height = 0;

    for img in loaded_images {
        let h = img.height();
        // 如果当前组已经有图，且加上新图会超高，则把当前组封存，开新组
        if !current_group.is_empty() && (current_height + h + padding > max_height_limit) {
            groups.push(current_group);
            current_group = Vec::new();
            current_height = 0;
        }

        // 累加高度（如果是第一张不需要padding）
        if !current_group.is_empty() {
            current_height += padding;
        }
        current_height += h;
        current_group.push(img);
    }
    // 剩下的放入最后一组
    if !current_group.is_empty() {
        groups.push(current_group);
    }

    // 3. 执行合并并转 Base64
    let mut result_base64s = Vec::new();

    for group in groups {
        // 计算画布大小
        let total_h: u32 =
            group.iter().map(|i| i.height()).sum::<u32>() + (group.len() as u32 - 1) * padding;
        let max_w: u32 = group.iter().map(|i| i.width()).max().unwrap_or(0);

        let mut canvas = DynamicImage::new_rgba8(max_w, total_h);
        let mut y_offset = 0;

        for img in group {
            // 简单的左对齐拼接
            // 注意：copy_from 可能会失败如果超出边界，但在我们计算好的画布上一般不会
            let _ = canvas.copy_from(&img, 0, y_offset);
            y_offset += img.height() + padding;
        }

        // 限制最大宽度，防止图片过大
        let final_width = if max_w > 1024 { 1024 } else { max_w };
        let resized_canvas = if final_width < max_w {
            let scale_factor = final_width as f64 / max_w as f64;
            let final_height = (total_h as f64 * scale_factor) as u32;
            canvas.resize(
                final_width,
                final_height,
                image::imageops::FilterType::Triangle,
            )
        } else {
            canvas
        };

        // 转 Base64
        let mut buf = Vec::new();
        // 推荐用 JPEG 压缩体积，质量设为 80 左右
        resized_canvas.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)?;
        let b64 = general_purpose::STANDARD.encode(&buf);
        result_base64s.push(format!("data:image/jpeg;base64,{}", b64));
    }

    Ok(result_base64s)
}
