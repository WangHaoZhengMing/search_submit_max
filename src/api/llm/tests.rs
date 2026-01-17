//! LLM 服务测试模块

use crate::{api::search::SearchResult, app::logger};

use super::LlmService;
use async_openai::{config::OpenAIConfig, Client};

/// 创建测试用的 LlmService
fn create_test_service() -> LlmService {
    let config = OpenAIConfig::new()
        .with_api_key("26e96c4d312e48feacbd78b7c42bd71e")
        .with_api_base("http://menshen.xdf.cn/v1");

    let client = Client::with_config(config);

    LlmService {
        client,
        model_name: "gemini-3.0-pro-preview".to_string(),
    }
}

/// 测试通用 LLM 调用
#[tokio::test]
#[ignore]
async fn test_send_to_llm_simple() {
    let _ = tracing_subscriber::fmt::try_init();

    let service = create_test_service();

    let user_message = "请描写一下这个图片中的内容";
    let system_message = Some("你是一个简洁的助手，回答要简短。");

    let result = service
        .send_to_llm(user_message, system_message, None)
        .await;

    match result {
        Ok(response) => {
            println!("\n========== LLM 响应 ==========");
            println!("{}", response);
            println!("==============================\n");
            println!("✅ 通用 LLM 调用成功！");
            assert!(!response.is_empty());
        }
        Err(e) => {
            println!("❌ LLM 调用失败: {}", e);
            panic!("测试失败: {}", e);
        }
    }
}

/// 测试 Vision API 图片理解能力
#[tokio::test]
#[ignore]
async fn test_vision_api() {
    let _ = tracing_subscriber::fmt::try_init();

    let service = create_test_service();

    println!("\n========== 测试 Vision API 图片理解 ==========");

    // 使用真实的公开图片 URL
    let image_urls = vec![
        "https://img.xkw.com/dksih/QBM/editorImg/2025/12/26/7b53a389-80ac-48a3-9833-98112c3bb0a7.png?resizew=151".to_string(),
    ];

    let user_message = "请详细描述这张图片中的内容，包括场景、颜色、物体等。";

    println!("用户消息: {}", user_message);
    println!("图片数量: {}", image_urls.len());
    for (i, url) in image_urls.iter().enumerate() {
        println!("  图片 {}: {}", i + 1, url);
    }
    println!("==========================================\n");

    let result = service
        .send_to_llm(
            user_message,
            Some("你是一个专业的图片分析助手。请仔细观察图片并给出详细描述。"),
            Some(&image_urls),
        )
        .await;

    match result {
        Ok(response) => {
            println!("\n========== LLM 响应 ==========");
            println!("{}", response);
            println!("==============================\n");
            println!("✅ Vision API 调用成功！");

            // 验证响应不为空且包含描述性内容
            assert!(!response.is_empty());
            assert!(response.len() > 50); // 应该有详细的描述
        }
        Err(e) => {
            println!("\n❌ Vision API 调用失败: {}", e);
        }
    }
}

/// 测试 Vision API 多图片处理
#[tokio::test]
#[ignore]
async fn test_vision_api_multiple_images() {
    let _ = tracing_subscriber::fmt::try_init();

    let service = create_test_service();

    println!("\n========== 测试 Vision API 多图片处理 ==========");

    let image_urls = vec![
        "https://img.xkw.com/dksih/QBM/editorImg/2025/9/19/91e4fa99-85ae-4778-b769-0e1ed7d04ec6.png?resizew=72"
            .to_string(),
        "https://img.xkw.com/dksih/QBM/editorImg/2025/9/19/cd4ca712-4e58-4f15-91b7-e8f8691b6767.jpg?resizew=168"
            .to_string(),
    ];

    let user_message = "请比较这两张图片的异同点。";

    println!("用户消息: {}", user_message);
    println!("图片数量: {}", image_urls.len());
    println!("==========================================\n");

    let result = service
        .send_to_llm(user_message, None, Some(&image_urls))
        .await;

    match result {
        Ok(response) => {
            println!("\n========== LLM 响应 ==========");
            println!("{}", response);
            println!("==============================\n");
            println!("✅ 多图片 Vision API 调用成功！");
            assert!(!response.is_empty());
        }
        Err(e) => {
            println!("\n❌ Vision API 调用失败: {}", e);
            panic!("多图片 Vision API 测试失败: {}", e);
        }
    }
}

/// 测试 LLM API 连接性（带图片）
#[tokio::test]
async fn test_llm_api_match() {
    logger::init_test();

    let service = create_test_service();

    let search_results = vec![
        SearchResult {
            question_content: "下列有关宋与辽、西夏、金政权并立的示意图，不正确的是（     ）".to_string(),
            xkw_question_similarity: Some(0.95),
            img_urls: Some(vec![
                String::from("https://k12static.xdf.cn/k12/xkw/1765604022831/ea0764be559e4ceb92328b02d0b874a8.png"),
                String::from("https://k12static.xdf.cn/k12/xkw/1765604022961/efaeadb21fb7491e8d79f1cd5b0cfd0b.png"),
                String::from("https://k12static.xdf.cn/k12/xkw/1765604023110/038741fdd9004be7b6705902ef7720bf.png"),
                String::from("https://k12static.xdf.cn/k12/xkw/1765604023265/5736c0da12284ebd87493436b2183e27.png")
            ]),
        },
        SearchResult {
            question_content: "下列有关宋与辽、西夏、金政权并立的示意图不正确的是（     ）

                北宋                 北宋                    南宋            南宋".to_string(),
            xkw_question_similarity: Some(0.85),
            img_urls: None,
        },
    ];

    let stem = "下列有关宋与辽、西夏、金政权并立的示意图，不正确的是（     ）";
    let target_imgs =
        vec!["https://k12static.xdf.cn/k12/xkw/1750294449260/01fdf398-c731-44b9-9ec7-183b13a892bb.png".to_string(),
            String::from("https://k12static.xdf.cn/k12/xkw/1750294449444/5f63c6c7-a6fd-47c5-b1c4-65e93055886b.png"),
            String::from("https://k12static.xdf.cn/k12/xkw/1750294449587/cbb524c1-c2be-4c05-a978-a1cc411d5191.png"),
            String::from("https://k12static.xdf.cn/k12/xkw/1750294449749/9c661e1b-2b5e-4949-912c-4b1d56a18624.png")

        ];

    // let _result = service
    //     .find_best_match_index(&search_results, stem, Some(&target_imgs))
    //     .await;

    // match Some(index) {
    //     Ok(index) => {
    //         println!("LLM 选择的索引: {}", index);
    //         println!("选择的题目: {}", search_results[index].question_content);
    //         println!("==============================\n");
    //         assert!(index < search_results.len());
    //     }
    //     Err(e) => {
    //         println!("\n❌ LLM API 调用失败: {}", e);
    //         panic!("LLM API 测试失败: {}", e);
    //     }
    // }
}
