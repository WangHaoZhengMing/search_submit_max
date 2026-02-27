use tracing::{debug, warn};

use crate::api::search::SearchResult;

/// 构建用于题目匹配的消息
/// 注意：现在是 async fn
pub async fn build_send_messages(
    search_results: &[SearchResult],
    target_img: &str, 
) -> (String, String, Vec<String>) {
    let mut all_images_to_send = Vec::new();

    // 1. 添加目标题目截图（Image 1）
    all_images_to_send.push(target_img.to_string());

    // 记录当前图片索引（从 Image 2 开始给候选题目）
    let mut current_img_index = 1;

    // 2. 处理候选题目列表
    let mut candidates_vec = Vec::new();

    for (idx, result) in search_results.iter().enumerate() {
        let mut candidate = serde_json::json!({
            "index": idx,
            "content": &result.question_content,
            "similarity": result.xkw_question_similarity,
        });

        let img_ref_str;

        if let Some(c_imgs) = &result.img_urls {
            if !c_imgs.is_empty() {
                // 决定最终要发哪些图
                let final_imgs_for_this_candidate: Vec<String>;

                if c_imgs.len() > 1 {
                    // 尝试合并多张图片
                    match super::image_merger::smart_merge_images(c_imgs).await {
                        Ok(merged_imgs) => {
                            // 合并成功（可能返回 1 张，也可能因为太高返回多张）
                            debug!(
                                "候选题目 {} 的 {} 张图片合并为 {} 张",
                                idx,
                                c_imgs.len(),
                                merged_imgs.len()
                            );
                            final_imgs_for_this_candidate = merged_imgs;
                        }
                        Err(e) => {
                            // 降级策略：如果下载/合并失败，回退到原始 URL 列表
                            warn!(
                                "候选题目 {} 图片合并失败: {}, 回退到原始 {} 张图片",
                                idx,
                                e,
                                c_imgs.len()
                            );
                            final_imgs_for_this_candidate = c_imgs.to_vec();
                        }
                    }
                } else {
                    // 单张图片直接使用
                    final_imgs_for_this_candidate = c_imgs.to_vec();
                }

                // 将处理后的图片（Base64 或 URL）加入总列表
                all_images_to_send.extend(final_imgs_for_this_candidate.clone());

                // 计算索引范围
                let start = current_img_index + 1;
                let end = current_img_index + final_imgs_for_this_candidate.len();
                current_img_index = end;

                // 生成描述
                if start == end {
                    img_ref_str = format!("Image {}", start);
                } else {
                    img_ref_str = format!("Image {} 到 Image {}", start, end);
                }
            } else {
                img_ref_str = "无图片".to_string();
            }
        } else {
            img_ref_str = "无图片".to_string();
        }

        candidate["image_ref"] = serde_json::json!(img_ref_str);
        candidates_vec.push(candidate);
    }

    let candidates_json = serde_json::to_string_pretty(&candidates_vec).unwrap_or_default();

    // 3. 构1Prompt
    let system_message = "你是一个高精度的试题查重专家，擅长处理理科和文科题目。\
                         你的核心任务是区分【完全相同的原题】与【相似的改编题】。\
                         你需要具备极强的抗干扰能力，能忽略格式差异和OCR轻微错误，但对题目核心内容的变化极其敏感。".to_string();

    let user_message = format!(
        r#"请在【候选列表】中找出与【目标题目】属于同一道原题的选项。

【图片索引说明】
共发送 {} 张图片。请严格对应图片索引进行查看。

【目标题目】
图片索引：Image 1
（这是目标题目的完整截图，请仔细查看）

【候选题目列表】
{}

【核心判断逻辑 - 请针对题型采取不同策略】

**场景一：如果是理科题（数学/物理/化学等）**
1. **数值绝对匹配**：题干中的数字、公式系数、物理量必须完全一致。
   - 🚫 拒绝：数字由 "10" 变为 "20"，或 "最大值" 变为 "最小值"（这是改编题）。
2. **符号与图形**：变量名（x, y）和几何图结构必须一致。
3. 对于数学中的中心对称图形的是，你要好好查看图形细节，尤其是标注的角度、边长、特殊点（如重心、垂足）等。有不同的话就是改编题。你一定要严格区分这些细节，哪怕是很小的改动也可能导致答案完全不同。

**场景二：如果是文科题（语文/英语/历史/政治等）**
1. **文本指纹匹配**：
   - 重点核对：古诗文填空、英语阅读理解的原文、选择题的**具体选项内容**。
   - 🚫 拒绝：如果文章主题相同，但**设问方式不同**（例如问"主旨"变成了问"细节"），这是改编题。
   - 🚫 拒绝：英语完形填空如果挖空的**位置不同**，这是改编题。
2. **容错机制（针对 OCR 误差）**：
   - ✅ 允许：少量的错别字（如"形像"vs"形象"）、标点符号差异、换行位置不同。
   - ✅ 允许：题目中多出或少了"阅读下列材料..."这种通用指令语。
   - ❌ 禁止：关键名词、人名、地名、朝代发生变化。

**场景三：通用视觉标准**
- ✅ 允许：清晰度差异、水印、手写痕迹、排版差异。
- ✅ 允许：图片截图范围不同（部分题目可能包含题号，部分可能不包含）。
- ❌ 禁止：图片内容主体特征不一致（如地图、插图的主要元素）。

**【重要】**
- 请将目标题目截图中的内容与候选题目的文字内容（content字段）和图片进行对比。
- 候选题目的 content 字段已经包含了题目的文字内容，请与目标截图中的文字进行比对。
- 你也要对比图片的区别。

【最终决策】
- **匹配**：只有当题目的**核心考点、数据/文本主体、设问方式**均一致时，才视为匹配。
- **不匹配**：任何核心要素的改动（哪怕是很小的改动），只要改变了题目的答案或解法，均视为改编题，返回 None。

【输出格式】
- 找到严格匹配的原题：仅返回该候选项目的 `index` 数字（例如：0）。
- 未找到严格匹配：仅返回字符串 `None`。
- **严禁输出任何多余字符。**

再次强调！！！【输出格式】
- 找到严格匹配的原题：仅返回该候选项目的 `index` 数字（例如：0）。
- 未找到严格匹配：仅返回字符串 `None`。
- **严禁输出任何多余字符。**

你他妈要是多输出了其它东西我就炸你。
"#,
        all_images_to_send.len(),
        candidates_json
    );

    debug!("构建消息完成: 共 {} 张图片", all_images_to_send.len());
    debug!("候选题目数量: {}", search_results.len());

    (user_message, system_message, all_images_to_send)
}
