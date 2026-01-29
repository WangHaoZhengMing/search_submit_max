use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use futures::StreamExt;
use anyhow::{Context, Result};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{error, info}; 

use crate::api::submit::{submit_generated_question};
use crate::api::submit_paper::submit_paper;
use crate::app::data_subject::smart_find_subject_code;
use crate::app::models::Paper;
use crate::app::workflow::QuestionCtx;
use crate::app::workflow::process_single::{
    create_llm_service, process_single_question, submit_matched_question, submit_title_question,
};
use crate::app::workflow::process_single::result::BuildResult;
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

    let total = paths.len();
    let processed_cnt = Arc::new(AtomicUsize::new(0));

    futures::stream::iter(paths)
        .map(|path| {
            let processed_cnt = processed_cnt.clone();
            async move {
                info!("开始处理试卷: {:?}", path);

                if let Err(e) = process_single_paper(&path).await {
                    error!("试卷 {:?} 处理失败，跳过。错误: {:?}", path, e);
                }

                let current = processed_cnt.fetch_add(1, Ordering::SeqCst) + 1;
                let remaining = total.saturating_sub(current);
                let _ = std::fs::write(
                    "process.txt",
                    format!("已完成: {}\n待处理: {}\n总数: {}\n", current, remaining, total),
                );
            }
        })
        .buffer_unordered(80) // 同时处理 80 套试卷
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
    let subject_code: String = smart_find_subject_code(&paper.subject).ok_or_else(|| anyhow::anyhow!("无法识别科目代码"))?.to_string();
    let path_display = path.display().to_string();

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
        tasks.push((question, ctx));
    }

    enum SubmitAction {
        Title,
        Matched(Value),
        Generated(Value),
        None,
    }

    let results: Vec<_> = futures::stream::iter(tasks.into_iter().enumerate())
        .map(|item| async move {
            let (index, _) = item;
            if index > 0 {
                sleep(Duration::from_millis(500)).await;
            }
                item 
        })
        .buffered(1) 
        .map(|(index, (question, ctx))| {
            // 克隆必要的上下文
            let llm_service = llm_service.clone();
            let path_display = path_display.clone();

            async move {
                if ctx.is_title { Ok((index, question, ctx, SubmitAction::Title))
                } else {
                    let result = process_single_question(&question, &ctx, &llm_service, &question.stem).await;
                    match result {
                        Ok(BuildResult::Found { matched_data, .. }) => {
                            Ok((index, question, ctx, SubmitAction::Matched(matched_data)))
                        }
                        Ok(BuildResult::Generated { question: generated_data, .. }) => {
                            info!(target: "failed_questions", "{} 由 LLM 生成，试卷: {} | page_id: {} | 题号: {}", ctx.log_prefix(), path_display, ctx.paper_id, ctx.not_include_title_index);
                            Ok((index, question, ctx, SubmitAction::Generated(generated_data)))
                        }
                        Ok(BuildResult::ManualRequired { reason, .. }) => {
                            tracing::warn!(target: "failed_questions", "需人工 | 试卷: {} | page_id: {} | 题号: {} | 原因: {}", path_display, ctx.paper_id, ctx.not_include_title_index, reason);
                            Ok((index, question, ctx, SubmitAction::None))
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
        .buffer_unordered(23)
        .collect()
        .await;

    // 统计结果并准备提交
    let mut success_count = 0;
    let mut failure_count = 0;
    
    // 使用 filter_map 一次性完成 拆包 + 过滤 + 错误计数
    let mut valid_submissions: Vec<_> = results
        .into_iter()
        .filter_map(|res| match res {
            Ok((_, _, _, SubmitAction::None)) => {
                failure_count += 1;
                None
            }
            Ok(item) => Some(item),
            Err(_e) => {
                failure_count += 1;
                None
            }
        })
        .collect();

    // 按照原始 index 排序
    valid_submissions.sort_by_key(|(index, _, _, _)| *index);

    info!("试卷 {:?} 预处理完成，准备顺序提交 {} 个任务 (预处理失败: {})", 
        path, valid_submissions.len(), failure_count);

    // 顺序提交
    for (_index, question, ctx, action) in valid_submissions {
        let submit_result = match action {
            SubmitAction::Title => {
                submit_title_question(&question, &ctx).await
                    .map(|_| info!("{} 标题提交成功", ctx.log_prefix())) // 顺手打日志
            },
            
            // 合并 Matched 和 Generated，逻辑是一样的
            SubmitAction::Matched(ref data)  => {
                submit_matched_question(&ctx, data).await
                    .map(|_| info!("{} 题目提交成功", ctx.log_prefix()))
            },
             SubmitAction::Generated(ref data)  => {
                submit_generated_question(&ctx, data).await
                    .map(|_| info!("{} 题目提交成功", ctx.log_prefix()))
            },
            SubmitAction::None => Ok(()), 
        };

        match submit_result {
            Ok(_) => {
                if !matches!(action, SubmitAction::None) {
                    success_count += 1;
                }
            },
            Err(e) => {
                failure_count += 1; 
                let err_msg = format!("提交失败: {:?}", e);
                error!("{} {}", ctx.log_prefix(), err_msg);
                
                // 只有非 Title 的才记录到 failed_questions 业务日志表里？看你需求
                if !matches!(action, SubmitAction::Title) {
                     tracing::warn!(target: "failed_questions", "提交失败 | 试卷: {} | page_id: {} | 题号: {} | 原因: {:?}", 
                        path.display(), ctx.paper_id, ctx.not_include_title_index, e);
                }
            }
        }
    }

    info!("当前试卷全部完成: 成功提交 {}, 总失败 {}", success_count, failure_count);

    submit_paper(&page_id).await?;

    std::fs::remove_file(path).with_context(|| format!("删除文件失败: {:?}", path))?;
    info!("已删除试卷文件: {:?}", path);

    Ok(())
}
