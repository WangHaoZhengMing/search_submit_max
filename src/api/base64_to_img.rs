use base64::{engine::general_purpose, Engine as _};

/// 将 Base64 字符串解码为 PNG 图片字节数据
///
/// # 参数
/// - `base64_str`: Base64 编码的字符串，支持带或不带 data URL 前缀
///   (例如: "data:image/png;base64,iVBORw0KGgo..." 或 "iVBORw0KGgo...")
///
/// # 返回
/// - `Ok(Vec<u8>)`: PNG 图片的字节数据
/// - `Err`: 解码失败时的错误
///
/// # 示例
/// ```ignore
/// let base64_str = "data:image/png;base64,iVBORw0KGgo...";
/// let png_bytes = base64_to_png_img(base64_str)?;
/// ```
pub fn base64_to_png_img(base64_str: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {

    let base64_data = if base64_str.starts_with("data:") {
        // 这是一个 data URL，需要提取 base64 部分
        if let Some(base64_marker_pos) = base64_str.find("base64,") {
            // 跳过 "base64," (7个字符)
            &base64_str[(base64_marker_pos + 7)..]
        } else {
            // 格式不正确的 data URL，尝试找第一个逗号
            base64_str
                .find(',')
                .map(|pos| &base64_str[(pos + 1)..])
                .unwrap_or(base64_str)
        }
    } else {
        // 没有 data: 前缀，直接当作 base64 字符串
        base64_str
    };

    // 使用 STANDARD 引擎解码 Base64 字符串
    let img_data = general_purpose::STANDARD.decode(base64_data)?;

    Ok(img_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_to_png_with_prefix() {
        // 一个简单的 1x1 透明 PNG 的 base64
        let base64_with_prefix = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let result = base64_to_png_img(base64_with_prefix);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // PNG 文件必须以这个魔数开头
        assert_eq!(&bytes[0..4], b"\x89PNG");
    }

    #[test]
    fn test_base64_to_png_without_prefix() {
        let base64_without_prefix = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let result = base64_to_png_img(base64_without_prefix);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..4], b"\x89PNG");
    }
}