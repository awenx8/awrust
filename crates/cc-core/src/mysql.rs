//! MySQL 连接的初始化与多连接管理。

use std::time::Duration;

use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions, MySqlSslMode};

use crate::config::MysqlConfig;
use crate::error::{ConfigResult, Error, mask_url};

/// 把配置里的字符串 ssl_mode 映射到 sqlx 的枚举（无法识别时回退 Preferred）。
pub fn ssl_mode_from_str(s: &str) -> MySqlSslMode {
    match s.trim().to_ascii_lowercase().as_str() {
        "disabled" | "disable" | "off" => MySqlSslMode::Disabled,
        "required" | "require" => MySqlSslMode::Required,
        "verify-ca" | "verify_ca" => MySqlSslMode::VerifyCa,
        "verify-identity" | "verify_identity" => MySqlSslMode::VerifyIdentity,
        _ => MySqlSslMode::Preferred,
    }
}

/// 根据配置构造连接选项。
///
/// 配置以 `url` 连接串为主（如 `mysql://user:pass@host:3306/db`）；
/// `url` 为空时回退到逐字段配置。url 模式下非空的 host/user/password/database
/// 字段仍可覆盖 url 中对应的部分；ssl_mode 字段仅在字段模式下生效，
/// url 模式的 SSL 请在连接串 query 中指定（如 `?sslmode=required`）。
pub fn connect_options(cfg: &MysqlConfig) -> ConfigResult<MySqlConnectOptions> {
    let mut opts = if cfg.url.is_empty() {
        MySqlConnectOptions::new()
            .host(&cfg.host)
            .port(cfg.port)
            .username(&cfg.user)
            .password(&cfg.password)
            .ssl_mode(ssl_mode_from_str(&cfg.ssl_mode))
    } else {
        cfg.url.parse::<MySqlConnectOptions>().map_err(|e| {
            Error::ConfigValidation(format!("MySQL url 解析失败: `{}`（{e}）", cfg.url))
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
    if cfg.disable_sql_mode {
        opts = opts.no_engine_substitution(false).pipes_as_concat(false);
    }
    if !cfg.database.is_empty() {
        opts = opts.database(&cfg.database);
    }
    Ok(opts)
}

/// 用单个配置建立连接池。
pub async fn connect(cfg: &MysqlConfig) -> ConfigResult<MySqlPool> {
    let target = cfg
        .url()
        .map(mask_url)
        .unwrap_or_else(|| format!("{}:{}", cfg.host, cfg.port));
    tracing::info!(target = %target, "建立 MySQL 连接");
    let pool_options = MySqlPoolOptions::new()
        .max_connections(cfg.max_connections)
        .acquire_timeout(Duration::from_secs(cfg.acquire_timeout.into()))
        .idle_timeout(Duration::from_secs(cfg.idle_timeout.into()));

    let opts = connect_options(cfg)?;
    let pool = pool_options
        .connect_with(opts)
        .await
        .map_err(|e| Error::MysqlConnect {
            target: target.clone(),
            source: e,
        })?;
    tracing::info!(target = %target, "MySQL 连接建立成功");
    Ok(pool)
}

define_sql_pools! {
    /// 多个命名 MySQL 连接池的容器。
    MysqlPools, MySqlPool,
    label = "MySQL",
    config_field = mysql,
    connect_fn = connect,
    not_found = MysqlNotFound,
    health_check = MysqlHealthCheck,
}
