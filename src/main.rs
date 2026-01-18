#![recursion_limit = "256"]

mod api;
mod app;
mod config;

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = app::logger::init("logs", "search_submit");

    // 运行工作流 pipeline
    if let Err(e) = app::workflow::pipeline::run().await {
        tracing::error!("Pipeline 执行失败: {:?}", e);
        return Err(e);
    }
    info!("========== 所有试卷处理完成 ==========");

    Ok(())
}
