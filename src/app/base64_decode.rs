use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};

// 1. 定义一个 Trait
pub trait Base64Decode {
    /// 将 Base64 编码的字符串解码为普通字符串
    fn parse_as_base64(&self) -> Result<String>;
}

// 2. 为 String 类型实现这个 Trait
impl Base64Decode for String {
    fn parse_as_base64(&self) -> Result<String> {
        decode_base64(self)
    }
}

// 3. 为 &str (字符串切片) 也实现这个 Trait (这样字面量也能直接用)
impl Base64Decode for str {
    fn parse_as_base64(&self) -> Result<String> {
        decode_base64(self)
    }
}

// 内部辅助函数
fn decode_base64(input: &str) -> Result<String> {
    // 去除可能存在的换行符或空格
    let clean_input = input.trim();
    
    // 解码
    let decoded_bytes = general_purpose::STANDARD
        .decode(clean_input)
        .context("Base64 解码失败")?;

    // 将字节转换为 UTF-8 字符串
    let decoded_string = String::from_utf8(decoded_bytes)
        .context("解码后的内容不是有效的 UTF-8 字符串")?;

    Ok(decoded_string)
}

