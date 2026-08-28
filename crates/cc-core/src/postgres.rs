//! PostgreSQL 连接的初始化与多连接管理。

use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgSslMode};

use crate::config::PostgresConfig;
use crate::error::{ConfigResult, Error, mask_url};

/// 把配置里的字符串 ssl_mode 映射到 sqlx 的枚举（无法识别时回退 Prefer）。
pub fn ssl_mode_from_str(s: &str) -> PgSslMode {
    match s.trim().to_ascii_lowercase().as_str() {
        "disabled" | "disable" | "off" => PgSslMode::Disable,
        "allow" => PgSslMode::Allow,
        "required" | "require" => PgSslMode::Require,
        "verify-ca" | "verify_ca" => PgSslMode::VerifyCa,
        "verify-full" | "verify_full" | "verify-identity" | "verify_identity" => {
            PgSslMode::VerifyFull
        }
        _ => PgSslMode::Prefer,
    }
}

/// 根据配置构造连接选项。
///
/// 配置以 `url` 连接串为主（如 `postgres://user:pass@host:5432/db`）；
/// `url` 为空时回退到逐字段配置。url 模式下非空的 host/user/password/database
/// 字段仍可覆盖 url 中对应的部分；ssl_mode 字段仅在字段模式下生效，
/// url 模式的 SSL 请在连接串 query 中指定（如 `?sslmode=require`）。
pub fn connect_options(cfg: &PostgresConfig) -> ConfigResult<PgConnectOptions> {
    let mut opts = if cfg.url.is_empty() {
        PgConnectOptions::new()
            .host(&cfg.host)
            .port(cfg.port)
            .username(&cfg.user)
            .password(&cfg.password)
            .ssl_mode(ssl_mode_from_str(&cfg.ssl_mode))
    } else {
        cfg.url.parse::<PgConnectOptions>().map_err(|e| {
            Error::ConfigValidation(format!("PostgreSQL url 解析失败: `{}`（{e}）", cfg.url))
        })?
    };

    if !cfg.host.is_empty() {
        opts = opts.host(&cfg.host);
    }
    if !cfg.user.is_empty() {
        opts = opts.username(&cfg.user);
    }
    if !cfg.password.is_empty() {
        opts = opts.password(&cfg.password);
    }
    if !cfg.database.is_empty() {
        opts = opts.database(&cfg.database);
    }
    Ok(opts)
}

/// 用单个配置建立连接池。
pub async fn connect(cfg: &PostgresConfig) -> ConfigResult<PgPool> {
    let target = cfg
        .url()
        .map(mask_url)
        .unwrap_or_else(|| format!("{}:{}", cfg.host, cfg.port));
    tracing::info!(target = %target, "建立 PostgreSQL 连接");
    let pool_options = PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .acquire_timeout(Duration::from_secs(cfg.acquire_timeout.into()))
        .idle_timeout(Duration::from_secs(cfg.idle_timeout.into()));

    let opts = connect_options(cfg)?;
    let pool = pool_options
        .connect_with(opts)
        .await
        .map_err(|e| Error::PostgresConnect {
            target: target.clone(),
            source: e,
        })?;
    tracing::info!(target = %target, "PostgreSQL 连接建立成功");
    Ok(pool)
}

define_sql_pools! {
    /// 多个命名 PostgreSQL 连接池的容器。
    PostgresPools, PgPool,
    label = "PostgreSQL",
    config_field = postgres,
    connect_fn = connect,
    not_found = PostgresNotFound,
    health_check = PostgresHealthCheck,
}
