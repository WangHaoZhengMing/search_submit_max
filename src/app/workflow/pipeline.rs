use std::path::Path;

use anyhow::Result;
use tracing::{error, info, warn};

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
    let llm_service = create_llm_service(&config);
    let llm_service = std::sync::Arc::new(llm_service);

    let mut tasks = Vec::new();

    // 限制单张卷子内的并发度（例如限制同时查10个题，防止触发API限流）
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(1));

    for (index, question) in paper.stemlist.iter().enumerate() {
        if index > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }

        let sem_clone = semaphore.clone();
        let llm_service_clone = llm_service.clone();
        let question_clone = question.clone();

        let ctx = QuestionCtx {
            paper_id: paper
                .page_id
                .clone()
                .ok_or_else(|| anyhow::anyhow!("试卷缺少 page_id"))?,
            subject_code: "54".to_string(), //暂时写死数学
            stage: "3".to_string(),
            paper_index: 1,
            question_index: index + 1,
            is_title: question_clone.is_title,
            screenshot: question.screenshot.clone(),
        };

        // tokio::spawn 开启并发任务
        let task = tokio::spawn(async move {
            // 获取信号量许可（控制并发数）
            let _permit = sem_clone.acquire().await.unwrap();

            // 判断是否为标题题目
            if question_clone.is_title {
                info!("{} 检测到标题题目，直接提交", ctx.log_prefix());

                // 直接提交标题题目
                let result = submit_title_question(&question_clone, &ctx).await;

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
                // 非标题题目：执行核心逻辑：上传 -> 搜索 -> LLM -> 提交
                let result = process_single_question(
                    &question_clone,
                    &ctx,
                    &llm_service_clone,
                    &question_clone.stem,
                )
                .await;

                match result {
                    Ok(process_result) => {
                        info!(
                            "{} 处理完成 - 找到匹配: {}, 来源: {:?}",
                            ctx.log_prefix(),
                            process_result.found_match,
                            process_result.search_source
                        );
                        //如果匹配成功，则提交题目
                        if process_result.found_match {
                            info!("{} 匹配成功，开始提交匹配题目", ctx.log_prefix());
                            if let Some(matched_data) = process_result.matched_data {
                                let submit_res = submit_matched_question(
                                    &ctx,
                                    &matched_data,
                                    process_result.search_source.as_deref().unwrap_or("unknown"),
                                )
                                .await;
                                match submit_res {
                                    Ok(_) => info!("{} 匹配题目提交成功", ctx.log_prefix()),
                                    Err(e) => error!("{} 匹配题目提交失败: {:?}", ctx.log_prefix(), e),
                                }
                            }
                        }
                        Ok(())
                    }
                    Err(e) => {
                        error!("{} 处理失败: {:?}", ctx.log_prefix(), e);
                        Err(e)
                    }
                }
            }
        });
        tasks.push(task);
    }

    // 等待这张卷子的所有题目跑完
    // join_all 会等待所有 Future 完成
    let results = futures::future::join_all(tasks).await;

    // 检查结果：统计这张卷子成功多少，失败多少
    let mut success_count = 0;
    let mut failure_count = 0;

    for res in results {
        match res {
            Ok(Ok(_)) => {
                success_count += 1;
            }
            Ok(Err(e)) => {
                failure_count += 1;
                warn!("题目处理失败: {:?}", e);
            }
            Err(e) => {
                failure_count += 1;
                error!("任务执行失败（Panic）: {:?}", e);
            }
        }
    }

    info!(
        "试卷 {:?} 处理完成 - 成功: {}, 失败: {}",
        path, success_count, failure_count
    );

    Ok(())
}
