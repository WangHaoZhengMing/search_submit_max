use std::path::Path;
use std::sync::Arc;
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

    // 收集所有符合条件的试卷路径
    let mut paths = Vec::new();
    for entry in toml_files {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            paths.push(path);
        }
    }

    info!("扫描到 {} 个待处理试卷文件", paths.len());

    // 并发处理多套试卷
    // 注意：单张试卷内部已经是并发处理（23并发），所以这里控制试卷级别的并发数不宜过高
    // 设为 3，防止系统资源或 API 限制被撑爆
    futures::stream::iter(paths)
        .map(|path| async move {
            info!("开始处理试卷: {:?}", path);

            if let Err(e) = process_single_paper(&path).await {
                error!("试卷 {:?} 处理失败，跳过。错误: {:?}", path, e);
            }
        })
        .buffer_unordered(3) // 同时处理 3 套试卷
        .collect::<Vec<_>>()
        .await;

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
        // println!("{:?},{}",&ctx.question_index, &ctx.not_include_title_index);
        tasks.push((question, ctx));
    }

    enum SubmitAction {
        Title,
        Matched(serde_json::Value, String),
        None,
    }

    // 创建并发流 - 只负责处理，不负责提交
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
                    info!("{} 检测到标题题目，准备提交", ctx.log_prefix());
                    // 标题题目直接标记为需要提交Title
                    Ok((index, question, ctx, SubmitAction::Title))
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
                                    let source = process_result.search_source.unwrap_or_else(|| "unknown".to_string());
                                    Ok((index, question, ctx, SubmitAction::Matched(matched_data, source)))
                                } else {
                                    tracing::warn!(target: "failed_questions", "匹配成功但无数据 | 试卷: {} | page_id: {} | 题号: {}", path_display, ctx.paper_id, ctx.not_include_title_index);
                                    Ok((index, question, ctx, SubmitAction::None))
                                }
                            } else {
                                tracing::warn!(target: "failed_questions", "无匹配 | 试卷: {} | page_id: {} | 题号: {}", path_display, ctx.paper_id, ctx.not_include_title_index);
                                Ok((index, question, ctx, SubmitAction::None))
                            }
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
        .buffer_unordered(23) // 控制并发数为
        .collect()
        .await;

    // 统计结果并准备提交
    let mut success_count = 0;
    let mut failure_count = 0;
    
    // 收集成功的任务
    let mut valid_submissions = Vec::new();

    for res in results {
        match res {
            Ok(item) => valid_submissions.push(item),
            Err(_) => failure_count += 1,
        }
    }

    // 按照原始 index 排序，确保提交顺序
    valid_submissions.sort_by_key(|(index, _, _, _)| *index);

    info!("试卷 {:?} 处理阶段完成，开始按顺序提交 {} 个任务", path, valid_submissions.len());

    // 顺序提交
    for (_index, question, ctx, action) in valid_submissions {
        let result = match action {
            SubmitAction::Title => {
                let res = submit_title_question(&question, &ctx).await;
                match res {
                    Ok(_) => {
                        info!("{} 标题题目提交成功", ctx.log_prefix());
                        Ok(())
                    },
                    Err(e) => {
                         error!("{} 标题题目提交失败: {:?}", ctx.log_prefix(), e);
                         Err(e)
                    }
                }
            },
            SubmitAction::Matched(ref matched_data, ref source) => {
                let res = submit_matched_question(&ctx, &matched_data, &source).await;
                match res {
                    Ok(_) => {
                        info!("{} 匹配题目提交成功", ctx.log_prefix());
                        Ok(())
                    }
                    Err(e) => {
                        let err_msg = format!("提交失败: {:?}", e);
                        error!("{} {}", ctx.log_prefix(), err_msg);
                        tracing::warn!(target: "failed_questions", "提交失败 | 试卷: {} | page_id: {} | 题号: {} | 原因: {:?}", path.display(), ctx.paper_id, ctx.not_include_title_index, e);
                        Err(e)
                    }
                }
            },
            SubmitAction::None => {
                // 已经处理过，这里不做操作
                Ok(())
            }
        };

        if result.is_ok() {
            success_count += 1;
        } else {
            // 注意：这里算作失败吗？如果是None算成功，如果是提交失败算失败。
            // 上面的 result.is_ok() 对于 None 是 Ok(())。
            // 对于 Title/Matched 失败是 Err。
            if matches!(action, SubmitAction::None) {
                // None 不算成功也不算失败？或者已经在处理阶段记录了失败/无匹配
                // 这里我们暂且只统计"提交成功"的数量
            } else {
                failure_count += 1;
            }
        }
    }

    info!(
        "试卷 {:?} 流程结束 - 提交成功: {}, 流程失败/提交失败: {}",
        path, success_count, failure_count
    );

    Ok(())
}
