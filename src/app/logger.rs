use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, Layer, fmt::{self, layer}, layer::SubscriberExt, util::SubscriberInitExt};

/// 初始化日志系统
/// 返回 WorkerGuard，必须在 main 函数中一直持有它，否则文件日志不生效
pub fn init(log_dir: &str, file_prefix: &str) -> WorkerGuard {
    // 1. 设置文件 Appender (按天轮转: logs/app.log.)
    let file_appender = tracing_appender::rolling::daily(log_dir, file_prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 2. 定义文件层 (File Layer) - 只记录 ERROR
    // 使用 with_filter 单独控制这一层的过滤规则
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false) // 文件里不要颜色代码
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_filter(tracing::metadata::LevelFilter::ERROR); // 关键：只把 ERROR 写入文件

    // 3. 定义控制台层 (Console Layer) - 记录 INFO 及以上
    // 使用 RUST_LOG 环境变量，默认 info
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_file(false) // 控制台一般不需要文件名，太乱
        .with_line_number(false)
        .with_filter(env_filter);

    // 4. 注册所有层
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    // 必须返回 guard
    guard
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


