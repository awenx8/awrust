//! Redis 连接的初始化与多连接管理。

use std::collections::HashMap;

use ::redis::aio::ConnectionManager;
use ::redis::AsyncTypedCommands;
use ::redis::Client;

use crate::config::{Config, IntoConnectionName, RedisConfig};
use crate::error::mask_url;
use crate::error::{ConfigResult, Error};

/// 对 Redis URL 进行脱敏，隐藏密码部分。
///
/// `redis://:password@host:6379` → `redis://:****@host:6379`
/// Redis 连接状态信息。
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// 连接是否存活
    pub is_alive: bool,
}

/// Redis 连接封装，内部使用 `ConnectionManager` 实现自动重连与多路复用。
pub struct RedisConnection {
    /// 内部连接管理器（含自动重连与多路复用）
    manager: ConnectionManager,
}

impl Clone for RedisConnection {
    fn clone(&self) -> Self {
        Self {
            manager: self.manager.clone(),
        }
    }
}

impl std::fmt::Debug for RedisConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisConnection")
            .field("manager", &"<ConnectionManager>")
            .finish()
    }
}

impl RedisConnection {
    /// 创建新的 Redis 连接
    pub async fn new(cfg: &RedisConfig) -> ConfigResult<Self> {
        tracing::info!(url = %mask_url(&cfg.url), "创建 Redis 连接管理器");

        let client = Client::open(cfg.url.as_str()).map_err(|e| Error::RedisOpen {
            url: mask_url(&cfg.url),
            message: e.to_string(),
            source: e,
        })?;

        let manager = ConnectionManager::new(client)
            .await
            .map_err(|e| Error::RedisConnect {
                url: mask_url(&cfg.url),
                message: e.to_string(),
                source: e,
            })?;

        tracing::info!(url = %mask_url(&cfg.url), "Redis 连接管理器创建成功");
        Ok(Self { manager })
    }

    /// 获取连接管理器（clone 廉价，共享底层多路复用连接）。
    pub fn get_connection(&self) -> ConnectionManager {
        self.manager.clone()
    }

    /// 通过 `PING` 命令检查连接是否存活。
    pub async fn is_alive(&self) -> bool {
        let mut conn = self.manager.clone();
        conn.ping().await.is_ok()
    }
}

/// 用单个配置创建一个带自动重连的连接（可廉价 clone 共享）。
pub async fn connect(cfg: &RedisConfig) -> ConfigResult<RedisConnection> {
    RedisConnection::new(cfg).await
}

/// 多个命名 Redis 连接的管理器。
pub struct RedisManager {
    connections: HashMap<String, RedisConnection>,
}

impl std::fmt::Debug for RedisManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisManager")
            .field("connections", &self.connections.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl RedisManager {
    /// 为配置里声明的每个 `[redis.<名字>]` 建立连接。
    pub async fn from_config(cfg: &Config) -> ConfigResult<Self> {
        let mut connections = HashMap::new();
        for (name, rc) in &cfg.redis {
            tracing::info!(name = %name, "初始化 Redis 连接");
            connections.insert(name.clone(), connect(rc).await?);
        }
        tracing::info!(count = connections.len(), "所有 Redis 连接初始化完成");
        Ok(Self { connections })
    }

    /// 按名取连接。
    pub fn get(&self, name: impl IntoConnectionName) -> Option<&RedisConnection> {
        self.connections.get(&name.into_name())
    }

    /// 按名取连接，不存在时报错。
    pub fn require(&self, name: impl IntoConnectionName) -> ConfigResult<&RedisConnection> {
        let name = name.into_name();
        self.connections
            .get(&name)
            .ok_or_else(|| Error::RedisNotFound { name })
    }

    /// 获取默认连接（名字为 "default"）。
    pub fn default(&self) -> ConfigResult<&RedisConnection> {
        self.require("default")
    }

    /// 健康检查：对指定连接执行 `PING`。
    pub async fn ping(&self, name: impl IntoConnectionName) -> ConfigResult<()> {
        let name_str = name.into_name();
        let conn = self.require(&name_str)?;
        let mut cm = conn.get_connection();
        let _: String = cm.ping().await.map_err(|e| Error::RedisHealthCheck {
            name: name_str,
            message: e.to_string(),
            source: e,
        })?;
        Ok(())
    }

    /// 健康检查：检查所有连接。
    pub async fn ping_all(&self) -> ConfigResult<()> {
        for name in self.connections.keys() {
            self.ping(name.as_str()).await?;
        }
        Ok(())
    }

    /// 获取指定连接的状态信息。
    pub async fn stats(&self, name: impl IntoConnectionName) -> Option<ConnectionStats> {
        let conn = self.connections.get(&name.into_name())?;
        let is_alive = conn.is_alive().await;
        Some(ConnectionStats { is_alive })
    }

    /// 获取所有连接名称。
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.connections.keys().map(String::as_str)
    }

    /// 关闭所有连接。
    pub async fn shutdown(self) {
        tracing::info!("关闭所有 Redis 连接");
        let names: Vec<_> = self.connections.keys().cloned().collect();
        // 显式 drop 所有连接，立即释放资源
        drop(self.connections);
        for name in &names {
            tracing::debug!(name = %name, "已关闭 Redis 连接");
        }
        tracing::info!("所有 Redis 连接已关闭");
    }
}
