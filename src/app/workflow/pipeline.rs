use std::path::Path;
use std::sync::Arc;
use chrono::Duration;
use futures::StreamExt;
use anyhow::Result;
use tracing::{error, info}; // Remove unused 'warn'

use crate::app::models::Paper;
use crate::app::workflow::QuestionCtx;
use crate::app::workflow::process_single::{
    create_llm_service, process_single_question, submit_matched_question, submit_title_question,
};
use crate::config::AppConfig;

pub async fn run() -> Result<(), anyhow::Error> {
    let toml_files = std::fs::read_dir("output_toml")?;

    for entry in toml_files {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            info!("开始处理试卷: {:?}", path);

            if let Err(e) = process_single_paper(&path).await {
                error!("试卷 {:?} 处理失败，跳过。错误: {:?}", path, e);
            }
        }
    }

    Ok(())
}

// 处理单张试卷 (并发处理题目)
async fn process_single_paper(path: &Path) -> Result<()> {
    // 解析 TOML
    let content = std::fs::read_to_string(path)?;
    let paper: Paper = toml::from_str(&content)?;

    let config = AppConfig::load()?;
    let llm_service = Arc::new(create_llm_service(&config));
    let page_id = paper.page_id.clone().ok_or_else(|| anyhow::anyhow!("试卷缺少 page_id"))?;
    let subject_code = "54".to_string(); // 暂时写死数学
    let path_display = path.display().to_string();

    // 预处理题目上下文，确保序号正确
    let mut tasks = Vec::new();
    let mut pure_question_index = 0;

    for (i, question) in paper.stemlist.into_iter().enumerate() {
        let ctx = QuestionCtx {
            paper_id: page_id.clone(),
            subject_code: subject_code.clone(),
            stage: "3".to_string(),
            paper_index: 1,
            question_index: i + 1,
            is_title: question.is_title,
            screenshot: question.screenshot.clone(),
            not_include_title_index: if question.is_title { pure_question_index } else { pure_question_index += 1; pure_question_index },
        };
        println!("{:?},{}",&ctx.question_index, &ctx.not_include_title_index);
        tasks.push((question, ctx));
    }



    // 创建并发流
    let results: Vec<_> = futures::stream::iter(tasks.into_iter().enumerate())
        .map(|(index, (question, ctx))| {
            // 克隆必要的上下文
            let llm_service = llm_service.clone();
            let path_display = path_display.clone();

            async move {
                 // 延迟启动，避免瞬时并发过高
                if index > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                
                // 处理单个题目逻辑
                if ctx.is_title {
                    info!("{} 检测到标题题目，直接提交", ctx.log_prefix());
                    let result = submit_title_question(&question, &ctx).await;
                    match result {
                        Ok(_) => {
                            info!("{} 标题题目提交成功", ctx.log_prefix());
                            Ok(())
                        }
                        Err(e) => {
                            error!("{} 标题题目提交失败: {:?}", ctx.log_prefix(), e);
                            Err(e)
                        }
                    }
                } else {
                    let result = process_single_question(
                        &question, &ctx, &llm_service, &question.stem
                    ).await;

                    match result {
                        Ok(process_result) => {
                            info!(
                                "{} 处理完成 - 找到匹配: {}, 来源: {:?}",
                                ctx.log_prefix(),
                                process_result.found_match,
                                process_result.search_source
                            );
                            
                            if process_result.found_match {
                                if let Some(matched_data) = process_result.matched_data {
                                    let source = process_result.search_source.as_deref().unwrap_or("unknown");
                                    match submit_matched_question(&ctx, &matched_data, source).await {
                                        Ok(_) => info!("{} 匹配题目提交成功", ctx.log_prefix()),
                                        Err(e) => {
                                            let err_msg = format!("提交失败: {:?}", e);
                                            error!("{} {}", ctx.log_prefix(), err_msg);
                                            tracing::warn!(target: "failed_questions", "提交失败 | 试卷: {} | page_id: {} | 题号: {} | 原因: {:?}", path_display, ctx.paper_id, ctx.not_include_title_index, e);
                                        }
                                    }
                                }
                            } else {
                                tracing::warn!(target: "failed_questions", "无匹配 | 试卷: {} | page_id: {} | 题号: {}", path_display, ctx.paper_id, ctx.not_include_title_index);
                            }
                            Ok(())
                        }
                        Err(e) => {
                            error!("{} 处理失败: {:?}", ctx.log_prefix(), e);
                            tracing::warn!(target: "failed_questions", "流程异常 | 试卷: {} | page_id: {} | 题号: {} | 原因: {:?}", path_display, ctx.paper_id, ctx.not_include_title_index, e);
                            Err(e)
                        }
                    }
                }
            }
        })
        .buffer_unordered(23) // 控制并发数为 20
        .collect()
        .await;

    // 统计结果
    let mut success_count = 0;
    let mut failure_count = 0;

    for res in results {
        match res {
            Ok(_) => success_count += 1,
            Err(_) => failure_count += 1,
        }
    }

    info!(
        "试卷 {:?} 处理完成 - 成功: {}, 失败: {}",
        path, success_count, failure_count
    );

    Ok(())
}
