use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, layer},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// 初始化日志系统
/// 返回 WorkerGuard，必须在 main 函数中一直持有它，否则文件日志不生效
/// 现在返回 Vec<WorkerGuard> 以管理多个文件输出的 guard
pub fn init(log_dir: &str, file_prefix: &str) -> Vec<WorkerGuard> {
    let mut guards = Vec::new();

    // === 1. 系统错误日志 (logs/search_submit.YYYY-MM-DD) ===
    // 记录所有系统级别的 ERROR
    let sys_appender = tracing_appender::rolling::daily(log_dir, file_prefix);
    let (sys_writer, sys_guard) = tracing_appender::non_blocking(sys_appender);
    guards.push(sys_guard);

    let sys_layer = fmt::layer()
        .with_writer(sys_writer)
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_filter(tracing::metadata::LevelFilter::ERROR);

    // === 2. 失败题目清单日志 (logs/failed_questions.YYYY-MM-DD) ===
    // 专门记录 target="failed_questions" 的日志
    let fail_appender = tracing_appender::rolling::daily(log_dir, "failed_questions");
    let (fail_writer, fail_guard) = tracing_appender::non_blocking(fail_appender);
    guards.push(fail_guard);

    // 自定义过滤器：只接受 target 为 "failed_questions" 的日志
    let fail_layer = fmt::layer()
        .with_writer(fail_writer)
        .with_ansi(false)
        .with_file(false) // 清单文件不需要代码行号
        .with_line_number(false)
        .with_target(false) // 不需要显示 target 名字
        // .with_format(fmt::format().compact()) // 使用紧凑格式
        .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            metadata.target() == "failed_questions"
        }));

    // === 3. 控制台输出 ===
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_file(false)
        .with_line_number(false)
        .with_filter(env_filter);

    // 注册所有层
    tracing_subscriber::registry()
        .with(console_layer)
        .with(sys_layer)
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
