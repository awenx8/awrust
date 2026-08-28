//! 优雅关闭管理器：收集异步关闭回调，等待 OS 信号后按注册逆序执行。
//!
//! # 示例
//!
//! ```rust,no_run
//! use cc_core::shutdown::GracefulShutdown;
//!
//! # async fn example() {
//! let mut shutdown = GracefulShutdown::new();
//!
//! // 注册自定义关闭任务
//! shutdown.register("custom-task", async {
//!     println!("执行自定义清理...");
//! });
//!
//! // 等待 OS 信号并执行所有关闭回调
//! shutdown.wait_for_signal().await;
//! # }
//! ```

use std::future::Future;
use std::pin::Pin;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// 优雅关闭管理器，收集异步关闭回调，等待 OS 信号后按注册逆序执行。
pub struct GracefulShutdown {
    hooks: Vec<(&'static str, BoxFuture)>,
}

impl Default for GracefulShutdown {
    fn default() -> Self {
        Self::new()
    }
}

impl GracefulShutdown {
    /// 创建一个新的优雅关闭管理器。
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// 注册一个异步关闭任务。
    ///
    /// `name` 用于日志标识，`future` 为关闭时执行的异步任务。
    /// 关闭时按注册的**逆序**执行（后注册的先执行）。
    pub fn register<F>(&mut self, name: &'static str, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.hooks.push((name, Box::pin(future)));
    }

    /// 注册 MySQL 连接池的优雅关闭。
    ///
    /// 将 `MysqlPools::shutdown()` 纳入关闭流程。
    #[cfg(feature = "mysql")]
    pub fn register_mysql_pools(&mut self, pools: crate::mysql::MysqlPools) {
        self.register("mysql-pools", async move {
            pools.shutdown().await;
        });
    }

    /// 注册 PostgreSQL 连接池的优雅关闭。
    ///
    /// 将 `PostgresPools::shutdown()` 纳入关闭流程。
    #[cfg(feature = "postgres")]
    pub fn register_postgres_pools(&mut self, pools: crate::postgres::PostgresPools) {
        self.register("postgres-pools", async move {
            pools.shutdown().await;
        });
    }

    /// 注册 Redis 连接管理器的优雅关闭。
    ///
    /// 将 `RedisManager::shutdown()` 纳入关闭流程。
    #[cfg(feature = "redis")]
    pub fn register_redis_manager(&mut self, manager: crate::redis::RedisManager) {
        self.register("redis-manager", async move {
            manager.shutdown().await;
        });
    }

    /// 监听 OS 信号（SIGTERM / SIGINT），收到信号后执行所有关闭回调。
    ///
    /// 关闭回调按注册的**逆序**依次执行（后注册的先执行）。
    pub async fn wait_for_signal(self) {
        use tokio::signal;

        let ctrl_c = signal::ctrl_c();

        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal as unix_signal};
            let mut sigterm =
                unix_signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

            tokio::select! {
                _ = ctrl_c => {
                    tracing::info!("收到 SIGINT (Ctrl+C)，开始优雅关闭...");
                }
                _ = sigterm.recv() => {
                    tracing::info!("收到 SIGTERM，开始优雅关闭...");
                }
            }
        }

        #[cfg(not(unix))]
        {
            ctrl_c.await.ok();
            tracing::info!("收到 Ctrl+C，开始优雅关闭...");
        }

        self.execute_hooks().await;
    }

    /// 不等待信号，直接执行所有已注册的关闭回调。
    ///
    /// 关闭回调按注册的**逆序**依次执行（后注册的先执行）。
    pub async fn shutdown(self) {
        tracing::info!("开始执行优雅关闭...");
        self.execute_hooks().await;
    }

    /// 按注册逆序执行所有 hook。
    async fn execute_hooks(self) {
        let hooks = self.hooks;
        let total = hooks.len();
        tracing::info!(total = total, "共注册 {total} 个关闭任务");

        // 逆序执行：后注册的先关闭
        for (i, (name, future)) in hooks.into_iter().rev().enumerate() {
            tracing::info!(
                name = %name,
                step = i + 1,
                total = total,
                "执行关闭任务"
            );
            future.await;
            tracing::debug!(name = %name, "关闭任务完成");
        }

        tracing::info!("所有关闭任务已完成");
    }
}
