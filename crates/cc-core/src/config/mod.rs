//! 配置系统：配置来源**唯一**为 **数据库**。
//!
//! - **数据库（默认，`config-db` feature）** — 全部配置集中存储在 PostgreSQL 的 `app_config`
//!   统一配置表，由 `ConfigBuilder::from_database(url)` 或 `ConfigBuilder::auto()` 读取。
//!   `auto()` 使用 `APP_CONFIG_DATABASE_URL` / `CC_CONFIG_DB_URL` 环境变量定位引导连接，
//!   连接该库的 `app_config` 表读取其余全部配置；数据库为**唯一**结构化数据源。
//!
//! 除引导连接串外，环境变量**不**参与任何配置覆盖；运行模式、各连接参数与 tracing
//! 配置均只来自 `app_config` 表。
//!
//! # 数据库配置表结构
//!
//! ```text
//! app_config(group_name, key, value, ...)
//! ```
//!
//! 分组到 cc-core 配置的映射约定见 `config-db` feature 下的数据库加载模块文档。
//!
//! # 引导连接串环境变量
//!
//! ```text
//! APP_CONFIG_DATABASE_URL=postgres://postgres:secret@127.0.0.1:5432/configdb
//! CC_CONFIG_DB_URL=postgres://postgres:secret@127.0.0.1:5432/configdb
//! ```

#[cfg(feature = "config-db")]
mod db;
mod mysql;
mod postgres;
mod redis;
mod tracing;

pub use mysql::{MysqlConfig, MysqlConfigBuilder};
pub use postgres::{PostgresConfig, PostgresConfigBuilder};
pub use redis::{RedisConfig, RedisConfigBuilder};
pub use tracing::{TracingConfig, TracingConfigBuilder};

use std::collections::HashMap;

use serde::Deserialize;

use crate::error::{ConfigResult, Error};

// ──────────────────────────────────────────────
// 数据库配置值
// ──────────────────────────────────────────────

/// 全部配置的分组键值存储：`group_name -> (key -> 值)`。
///
/// 配置值统一以字符串保存，读取时按需强转（见 `Config::get_int` / `get_bool`）。
pub type ConfigStore = std::collections::HashMap<String, std::collections::HashMap<String, String>>;

/// 解析字符串为布尔值（true/1/yes/on 与 false/0/no/off）。
pub(crate) fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

// ──────────────────────────────────────────────
// 连接名抽象
// ──────────────────────────────────────────────

/// 连接名的抽象，用户可为枚举实现此 trait 以获得编译时检查。
pub trait IntoConnectionName {
    fn into_name(self) -> String;
}

impl IntoConnectionName for String {
    fn into_name(self) -> String {
        self
    }
}

impl IntoConnectionName for &String {
    fn into_name(self) -> String {
        self.clone()
    }
}

impl IntoConnectionName for &str {
    fn into_name(self) -> String {
        self.to_string()
    }
}

// ──────────────────────────────────────────────
// 验证 trait
// ──────────────────────────────────────────────

/// 配置项验证。`Config::build()` 会自动调用。
pub trait Validate {
    fn validate(&self) -> ConfigResult<()>;
}

// ──────────────────────────────────────────────
// Config — 顶层容器
// ──────────────────────────────────────────────

/// 整个配置：多个命名 PostgreSQL / MySQL / Redis 连接 + Tracing 日志配置。
///
/// 当从数据库（`config-db` feature）加载时，`store` 同时保留 `app_config`
/// 表中所有分组的强类型键值，可通过 `get_*()` 方法程序化读取。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    /// 当前运行模式（如 "dev"、"online"），由 ConfigBuilder 设置，不参与序列化。
    #[serde(skip)]
    pub mode: Option<String>,
    #[serde(default)]
    pub postgres: HashMap<String, PostgresConfig>,
    #[serde(default)]
    pub mysql: HashMap<String, MysqlConfig>,
    #[serde(default)]
    pub redis: HashMap<String, RedisConfig>,
    #[serde(default)]
    pub tracing: TracingConfig,
    /// 来自数据库 `app_config` 表的全部分组键值（仅 `config-db` 加载时填充）。
    #[serde(skip)]
    pub store: ConfigStore,
}

impl Config {
    /// 获取当前运行模式。
    pub fn mode(&self) -> Option<&str> {
        self.mode.as_deref()
    }

    /// 按名取 PostgreSQL 配置。
    pub fn postgres(&self, name: &str) -> Option<&PostgresConfig> {
        self.postgres.get(name)
    }

    /// 按名取 MySQL 配置。
    pub fn mysql(&self, name: &str) -> Option<&MysqlConfig> {
        self.mysql.get(name)
    }

    /// 按名取 Redis 配置。
    pub fn redis(&self, name: &str) -> Option<&RedisConfig> {
        self.redis.get(name)
    }

    /// 获取 Tracing 配置。
    pub fn tracing(&self) -> &TracingConfig {
        &self.tracing
    }

    /// 获取所有 PostgreSQL 连接名。
    pub fn postgres_names(&self) -> impl Iterator<Item = &str> {
        self.postgres.keys().map(String::as_str)
    }

    /// 获取所有 MySQL 连接名。
    pub fn mysql_names(&self) -> impl Iterator<Item = &str> {
        self.mysql.keys().map(String::as_str)
    }

    /// 获取所有 Redis 连接名。
    pub fn redis_names(&self) -> impl Iterator<Item = &str> {
        self.redis.keys().map(String::as_str)
    }

    // ── 数据库配置通用访问器（仅 `config-db` 加载时填充 `store`）──

    /// 按分组与键取原始配置值。
    pub fn get_value(&self, group: &str, key: &str) -> Option<&str> {
        self.store.get(group)?.get(key).map(String::as_str)
    }

    /// 按分组与键取整数值（按需解析）。
    pub fn get_int(&self, group: &str, key: &str) -> Option<i64> {
        self.get_value(group, key)?.trim().parse().ok()
    }

    /// 按分组与键取布尔值（按需解析）。
    pub fn get_bool(&self, group: &str, key: &str) -> Option<bool> {
        parse_bool(self.get_value(group, key)?)
    }

    /// 获取某个分组的全部键值（只读视图）。
    pub fn group(&self, group: &str) -> Option<&HashMap<String, String>> {
        self.store.get(group)
    }
}

impl Validate for Config {
    fn validate(&self) -> ConfigResult<()> {
        for (name, pc) in &self.postgres {
            pc.validate()
                .map_err(|e| Error::ConfigValidation(format!("PostgreSQL[{}]: {}", name, e)))?;
        }
        for (name, mc) in &self.mysql {
            mc.validate()
                .map_err(|e| Error::ConfigValidation(format!("MySQL[{}]: {}", name, e)))?;
        }
        for (name, rc) in &self.redis {
            rc.validate()
                .map_err(|e| Error::ConfigValidation(format!("Redis[{}]: {}", name, e)))?;
        }
        self.tracing
            .validate()
            .map_err(|e| Error::ConfigValidation(format!("Tracing: {}", e)))?;
        Ok(())
    }
}

// ──────────────────────────────────────────────
// ConfigBuilder
// ──────────────────────────────────────────────

/// 配置构建器：从数据库（`config-db`）读取全部配置，或以编程方式构建。
///
/// 从数据库读取全部配置（推荐入口）：
/// ```rust,no_run
/// use cc_core::{ConfigBuilder, ConfigResult};
///
/// # async fn run() -> ConfigResult<()> {
/// let cfg = ConfigBuilder::auto().await?;
/// # Ok(())
/// # }
/// ```
pub struct ConfigBuilder {
    postgres: HashMap<String, PostgresConfig>,
    mysql: HashMap<String, MysqlConfig>,
    redis: HashMap<String, RedisConfig>,
    tracing: TracingConfig,
}

impl ConfigBuilder {
    /// 创建空 ConfigBuilder。
    pub fn empty() -> Self {
        Self {
            postgres: HashMap::new(),
            mysql: HashMap::new(),
            redis: HashMap::new(),
            tracing: TracingConfig::default(),
        }
    }

    /// 从 PostgreSQL 的 `app_config` 统一配置表读取全部配置。
    ///
    /// 仅在启用 `config-db` feature 时可用；连接串指向承载 `app_config` 表的数据库。
    #[cfg(feature = "config-db")]
    pub async fn from_database(url: &str) -> ConfigResult<Config> {
        let store = db::load_config_store(url).await?;
        let cfg = db::build_config(store, None)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// 自动选择配置来源（推荐入口）：数据库为**唯一**结构化数据源。
    ///
    /// 1. 从 `APP_CONFIG_DATABASE_URL` / `CC_CONFIG_DB_URL` 环境变量读取引导连接串；
    /// 2. 连接该库的 `app_config` 表读取全部配置（含运行模式、各连接与 tracing 配置）。
    #[cfg(feature = "config-db")]
    pub async fn auto() -> ConfigResult<Config> {
        let url = db::config_database_url().ok_or(crate::error::Error::ConfigDbUrlMissing)?;
        let cfg = Self::from_database(&url).await?;
        Ok(cfg)
    }

    /// 程序化添加 / 覆盖单个 PostgreSQL 连接。
    pub fn with_postgres(
        mut self,
        name: impl Into<String>,
        f: impl FnOnce(PostgresConfigBuilder) -> PostgresConfigBuilder,
    ) -> Self {
        let name = name.into();
        ::tracing::debug!(name = %name, "配置 PostgreSQL 连接");
        let base = self.postgres.remove(&name).unwrap_or_default();
        let cfg = f(PostgresConfigBuilder(base)).0;
        self.postgres.insert(name, cfg);
        self
    }

    /// 程序化添加 / 覆盖单个 MySQL 连接。
    pub fn with_mysql(
        mut self,
        name: impl Into<String>,
        f: impl FnOnce(MysqlConfigBuilder) -> MysqlConfigBuilder,
    ) -> Self {
        let name = name.into();
        ::tracing::debug!(name = %name, "配置 MySQL 连接");
        let base = self.mysql.remove(&name).unwrap_or_default();
        let cfg = f(MysqlConfigBuilder(base)).0;
        self.mysql.insert(name, cfg);
        self
    }

    /// 程序化添加 / 覆盖单个 Redis 连接。
    pub fn with_redis(
        mut self,
        name: impl Into<String>,
        f: impl FnOnce(RedisConfigBuilder) -> RedisConfigBuilder,
    ) -> Self {
        let name = name.into();
        ::tracing::debug!(name = %name, "配置 Redis 连接");
        let base = self.redis.remove(&name).unwrap_or_default();
        let cfg = f(RedisConfigBuilder(base)).0;
        self.redis.insert(name, cfg);
        self
    }

    /// 程序化添加 / 覆盖 Tracing 配置。
    pub fn with_tracing(
        mut self,
        f: impl FnOnce(TracingConfigBuilder) -> TracingConfigBuilder,
    ) -> Self {
        ::tracing::debug!("配置 Tracing");
        let base = std::mem::take(&mut self.tracing);
        self.tracing = f(TracingConfigBuilder(base)).0;
        self
    }

    /// 构建最终配置并验证。
    pub fn build(self) -> ConfigResult<Config> {
        let cfg = Config {
            mode: None,
            postgres: self.postgres,
            mysql: self.mysql,
            redis: self.redis,
            tracing: self.tracing,
            store: ConfigStore::new(),
        };
        ::tracing::info!(
            mode = ?cfg.mode,
            postgres_count = cfg.postgres.len(),
            mysql_count = cfg.mysql.len(),
            redis_count = cfg.redis.len(),
            tracing_level = %cfg.tracing.level,
            tracing_format = %cfg.tracing.format,
            "配置构建完成"
        );
        cfg.validate()?;
        Ok(cfg)
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mysql_names_iterator() -> ConfigResult<()> {
        let cfg = ConfigBuilder::empty()
            .with_mysql("primary", |m| {
                m.host("h1").user("u").password("p").database("d")
            })
            .with_mysql("replica", |m| {
                m.host("h2").user("u").password("p").database("d")
            })
            .build()?;

        let mut names: Vec<_> = cfg.mysql_names().collect();
        names.sort();
        assert_eq!(names, vec!["primary", "replica"]);
        Ok(())
    }

    #[test]
    fn postgres_names_iterator() -> ConfigResult<()> {
        let cfg = ConfigBuilder::empty()
            .with_postgres("primary", |p| {
                p.host("h1").user("u").password("p").database("d")
            })
            .with_postgres("replica", |p| {
                p.host("h2").user("u").password("p").database("d")
            })
            .build()?;

        let mut names: Vec<_> = cfg.postgres_names().collect();
        names.sort();
        assert_eq!(names, vec!["primary", "replica"]);
        Ok(())
    }
}
