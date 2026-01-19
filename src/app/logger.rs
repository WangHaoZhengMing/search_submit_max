use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, layer},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// 初始化日志系统
pub fn init(log_dir: &str, _file_prefix: &str) -> Vec<WorkerGuard> {
    let mut guards = Vec::new();

    // === 2. 失败题目清单日志 (logs/failed_questions.YYYY-MM-DD) ===
    // 专门记录 target="failed_questions" 的日志
    let fail_appender = tracing_appender::rolling::daily(log_dir, "failed_questions");
    let (fail_writer, fail_guard) = tracing_appender::non_blocking(fail_appender);
    guards.push(fail_guard);

    // 文件层过滤器：只允许 target 为 "failed_questions" 的通过
    let fail_layer = fmt::layer()
        .with_writer(fail_writer)
        .with_ansi(false) // 文件里不要颜色
        .with_file(false) 
        .with_line_number(false)
        .with_target(false) 
        .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            metadata.target() == "failed_questions"
        }));

    // === 3. 控制台输出 ===
    // 关键修改在这里：
    // 1. 获取默认的环境变量配置（比如 RUST_LOG=debug）或默认为 "info"
    // 2. 强制添加一条指令：failed_questions=off
    // 这样控制台层会接收所有日志，但唯独屏蔽掉 failed_questions
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive("failed_questions=off".parse().expect("Directive parse failed"));

    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_file(false)
        .with_line_number(false)
        .with_filter(env_filter); // 应用上面修改后的过滤器

    // 注册所有层
    tracing_subscriber::registry()
        .with(console_layer)
        .with(fail_layer)
        .init();

    guards
}
#[allow(dead_code)]
pub fn init_test() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            layer()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_target(false),
        )
        .init();
}
