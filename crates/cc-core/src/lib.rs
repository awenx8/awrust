//! # cc-core
//!
//! 公共核心库：数据库 + 环境变量配置系统 + PostgreSQL / MySQL / Redis 连接管理 + Tracing 日志初始化 + HTTP 客户端 + 优雅关闭。
//!
//! ## 特性
//!
//! - **配置加载（config-db，默认）** — 全部配置集中存储在 PostgreSQL 的 `app_config` 统一配置表，支持 `ConfigBuilder::from_database` / `auto()`；环境变量（`APP_CONFIG_DATABASE_URL` 等）作为引导与最高优先级覆盖
//! - **PostgreSQL 连接池（默认）** — 多命名连接池管理，支持健康检查和优雅关闭
//! - **MySQL 连接池（可选）** — 多命名连接池管理，支持健康检查和优雅关闭
//! - **Redis 连接管理** — 多命名连接管理，支持自动重连和多路复用
//! - **Tracing 初始化** — 从配置读取日志级别和输出格式（json/pretty），一键初始化
//! - **HTTP 客户端** — 基于 reqwest 的薄封装，支持 base_url、超时、默认请求头
//! - **优雅关闭** — 注册回调式关闭管理器，内置 PostgreSQL / MySQL / Redis 便捷注册 + OS 信号监听
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use cc_core::{ConfigBuilder, ConfigResult};
//!
//! # async fn run() -> ConfigResult<()> {
//! let config = ConfigBuilder::auto().await?;
//! # Ok(())
//! # }
//! ```

pub mod error;

pub mod config;
pub use config::{Config, ConfigBuilder, ConfigStore, PostgresConfig, RedisConfig, TracingConfig};
pub use config::{IntoConnectionName, Validate};
pub use config::{MysqlConfig, MysqlConfigBuilder};
pub use config::{PostgresConfigBuilder, RedisConfigBuilder, TracingConfigBuilder};

pub mod shutdown;
pub use shutdown::GracefulShutdown;

#[macro_use]
mod sql_pools;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mysql")]
pub mod mysql;

#[cfg(feature = "redis")]
pub mod redis;

#[cfg(feature = "tracing-init")]
pub mod tracing;

#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "http")]
pub use http::{HttpClient, HttpClientBuilder};

pub use error::{ConfigResult, Error};
