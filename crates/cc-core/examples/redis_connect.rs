use cc_core::{redis::RedisManager, ConfigBuilder, IntoConnectionName};
use redis::AsyncTypedCommands;

enum RedisName {
    Default,
}

impl IntoConnectionName for RedisName {
    fn into_name(self) -> String {
        match self {
            Self::Default => "default".into(),
        }
    }
}

#[tokio::main]
async fn main() -> cc_core::ConfigResult<()> {
    // 从配置文件读取 PostgreSQL 引导连接，其余配置从 app_config 表加载
    let config = ConfigBuilder::auto().await?;

    cc_core::tracing::init_tracing(&config.tracing)?;

    let manager = RedisManager::from_config(&config).await?;
    let conn = manager.require(RedisName::Default)?;

    let mut cm = conn.get_connection();
    let pong: String = cm.ping().await?;
    println!("PING: {pong}");

    // 批量健康检查
    manager.ping_all().await?;
    println!("所有 Redis 连接健康检查通过");

    Ok(())
}
