//! 优雅关闭示例：初始化 PostgreSQL + Redis 连接，注册到 GracefulShutdown，
//! 然后等待 OS 信号（Ctrl+C / SIGTERM）自动逆序关闭所有资源。
//!
//! 运行：`cargo run --example graceful_shutdown`
//!
//! 配置来源：仓库根目录 `.env` 中的 `APP_CONFIG_DATABASE_URL`
//! （承载 `app_config` 统一配置表的 PostgreSQL 数据库）。

use cc_core::postgres::PostgresPools;
use cc_core::redis::RedisManager;
use cc_core::{ConfigBuilder, GracefulShutdown};

#[tokio::main]
async fn main() -> cc_core::ConfigResult<()> {
    // 0. 加载 `.env`，提供引导连接串
    dotenvy::dotenv().ok();

    // 1. 加载配置：从 `.env` 的 APP_CONFIG_DATABASE_URL 读取引导连接串，
    //    连接其 app_config 表加载全部配置。
    let config = ConfigBuilder::auto().await?;

    // 初始化 tracing 日志
    cc_core::tracing::init_tracing(&config.tracing)?;

    // 2. 初始化连接池
    let postgres_pools = PostgresPools::from_config(&config).await?;
    let redis_manager = RedisManager::from_config(&config).await?;

    // 3. 健康检查
    postgres_pools.ping_all().await?;
    redis_manager.ping_all().await?;
    println!("所有连接就绪");

    // 4. 注册优雅关闭
    let mut shutdown = GracefulShutdown::new();

    // 注册自定义清理任务（先注册的最后关闭）
    shutdown.register("custom-cleanup", async {
        println!("执行自定义业务清理...");
        // 例如：取消后台任务、刷写缓冲区、释放文件锁等
    });

    // 注册 Redis 连接关闭
    shutdown.register_redis_manager(redis_manager);

    // 注册 PostgreSQL 连接池关闭（最后注册，最先关闭）
    shutdown.register_postgres_pools(postgres_pools);

    // 5. 模拟业务运行
    println!("服务运行中，按 Ctrl+C 触发优雅关闭...");

    // 6. 等待 OS 信号，收到后按注册逆序执行：
    //    postgres-pools → redis-manager → custom-cleanup
    shutdown.wait_for_signal().await;

    println!("服务已安全退出");
    Ok(())
}
