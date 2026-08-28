//! Tracing 日志初始化，根据配置设置全局 subscriber。

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::TracingConfig;
use crate::error::{ConfigResult, Error};

/// 根据配置初始化 tracing subscriber（全局只调用一次）。
///
/// - `cfg.level` 用于设置日志级别过滤器，支持 `RUST_LOG` 环境变量覆盖
/// - `cfg.format` 决定输出格式：`"pretty"` 为彩色美化输出，`"json"` 为 JSON 结构化输出
///
/// # 错误
///
/// 重复调用会返回 `Error::TracingAlreadyInit` 错误。
///
/// # 示例
///
/// ```rust,no_run
/// use cc_core::config::TracingConfig;
/// use cc_core::tracing::init_tracing;
/// use cc_core::ConfigResult;
///
/// fn main() -> ConfigResult<()> {
///     let cfg = TracingConfig {
///         level: "info".into(),
///         format: "pretty".into(),
///     };
///     init_tracing(&cfg)?;
///     Ok(())
/// }
/// ```
pub fn init_tracing(cfg: &TracingConfig) -> ConfigResult<()> {
    // 优先使用 RUST_LOG 环境变量，否则使用配置中的 level
    let filter = match EnvFilter::try_from_default_env() {
        Ok(f) => f,
        Err(_) => EnvFilter::try_new(&cfg.level)
            .map_err(|e| Error::TracingInvalidLevel(format!("{}: {}", cfg.level, e)))?,
    };

    let format_lower = cfg.format.to_ascii_lowercase();

    match format_lower.as_str() {
        "json" => {
            let layer = fmt::layer().json();
            tracing_subscriber::registry()
                .with(filter)
                .with(layer)
                .try_init()
                .map_err(|_| Error::TracingAlreadyInit)?;
        }
        _ => {
            // 默认使用 pretty 格式
            let layer = fmt::layer().pretty();
            tracing_subscriber::registry()
                .with(filter)
                .with(layer)
                .try_init()
                .map_err(|_| Error::TracingAlreadyInit)?;
        }
    }

    tracing::info!(
        level = %cfg.level,
        format = %cfg.format,
        "Tracing subscriber 初始化完成"
    );
    Ok(())
}
