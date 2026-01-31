use std::fs::OpenOptions;
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
use crate::app::workflow::{PaperQuestionsStatus, QuestionCtx};
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
        .buffer_unordered(30) // 同时处理 30 套试卷
        .collect::<Vec<_>>()
        .await;

    Ok(())
}


async fn process_single_paper(path: &Path) -> Result<()> {
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
                    let mut result = process_single_question(&question, &ctx, &llm_service, &question.stem).await;

                    let mut retry_count = 0;
                    while retry_count < 10 {
                        if let Ok(BuildResult::Found { .. }) = result {
                            break;
                        }
                        retry_count += 1;
                        result = process_single_question(&question, &ctx, &llm_service, &question.stem).await;
                    }

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
        .buffer_unordered(50)
        .collect()
        .await;
    // ==========================================
    // 2. 汇总状态 (用于最后的判断)
    // ==========================================
    let mut final_status: PaperQuestionsStatus = PaperQuestionsStatus {
        paper_id: page_id.clone(),
        paper_name: "".to_string(), 
        matched: vec![],
        generated: vec![],
        manual: vec![],
    };

    let mut has_processing_errors = false;

    // 填充状态
    for res in &results {
        match res {
            Ok((_, _, ctx, action)) => {
                let idx = ctx.not_include_title_index as i32;
                match action {
                    SubmitAction::Matched(_) => final_status.matched.push(idx),
                    SubmitAction::Generated(_) => final_status.generated.push(idx),
                    SubmitAction::None => final_status.manual.push(idx),
                    SubmitAction::Title => {}, 
                }
            }
            Err(_) => {
                has_processing_errors = true;
            }
        }
    }
    use std::io::Write;
     // ==========================================
    // 新增：导出到 logs/output.json (Append 模式)
    // ==========================================
    let is_imperfect = !final_status.generated.is_empty() 
        || !final_status.manual.is_empty() 
        || has_processing_errors;

    if is_imperfect {
        if let Ok(json_line) = serde_json::to_string(&final_status) {
            // 1. 确保目录存在
            if let Err(e) = std::fs::create_dir_all("./logs") {
                error!("创建日志目录失败: {:?}", e);
            } else {
                // 2. 以追加模式打开文件
                let file_result = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("./logs/output.json");

                match file_result {
                    Ok(mut file) => {
                        // 3. 写入 JSON 行
                        if let Err(e) = writeln!(file, "{}", json_line) {
                            error!("写入 output.json 失败: {:?}", e);
                        } else {
                            info!("已记录非全匹配试卷信息到 logs/output.json");
                        }
                    }
                    Err(e) => error!("打开 output.json 失败: {:?}", e),
                }
            }
        }
    }

    // ==========================================
    // 3. 准备并执行所有单题提交 (无论什么类型都提交)
    // ==========================================
    let mut success_count = 0;
    let mut failure_count = 0;
    
    let mut valid_submissions: Vec<_> = results
        .into_iter()
        .filter_map(|res| match res {
            Ok((_, _, _, SubmitAction::None)) => {
                // 人工处理的题目虽然不提交API，但也算作 failure 计数，用于日志
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

    valid_submissions.sort_by_key(|(index, _, _, _)| *index);

    info!("试卷 {:?} 预处理完成，准备顺序提交 {} 个任务", path, valid_submissions.len());

    for (_index, question, ctx, action) in valid_submissions {
        let submit_result = match action {
            SubmitAction::Title => submit_title_question(&question, &ctx).await,
            SubmitAction::Matched(ref data) => submit_matched_question(&ctx, data).await,
            SubmitAction::Generated(ref data) => submit_generated_question(&ctx, data).await,
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
                error!("{} 提交失败: {:?}", ctx.log_prefix(), e);
            }
        }
    }

    info!("单题提交结束: 成功 {}, 失败/跳过 {}", success_count, failure_count);

    // ==========================================
    // 4. 核心判断：是否提交整卷 (submit_paper)
    // ==========================================
    
    // 条件：没有 Generated，没有 Manual，没有处理错误
    let is_perfect_match = final_status.generated.is_empty() 
        && final_status.manual.is_empty() 
        && !has_processing_errors;

    if is_perfect_match {
        info!("试卷 {} 全匹配校验通过，执行整卷提交并删除文件", path_display);
        
        // 只有全匹配才提交整卷状态
        submit_paper(&page_id).await?;
        std::fs::remove_file(path).with_context(|| format!("删除文件失败: {:?}", path))?;
        info!("已删除试卷文件: {:?}", path);
    } else {
        info!(
            "试卷 {} 包含非匹配项，保留文件不提交整卷状态 (Generated: {}, Manual: {}, Errors: {})", 
            path_display, 
            final_status.generated.len(), 
            final_status.manual.len(),
            if has_processing_errors { "Yes" } else { "No" }
        );
        std::fs::remove_file(path).with_context(|| format!("删除文件失败: {:?}", path))?;
    }

    Ok(())
}
