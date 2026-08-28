use cc_core::{mysql::MysqlPools, ConfigBuilder, IntoConnectionName};

enum MysqlName {
    Default,
}

impl IntoConnectionName for MysqlName {
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

    let pools = MysqlPools::from_config(&config).await?;
    let pool = pools.require(MysqlName::Default)?;

    let version: (String,) = sqlx::query_as("SELECT VERSION()")
        .fetch_one(pool)
        .await
        .map_err(|e| cc_core::Error::MysqlPool { source: e })?;
    println!("MySQL: {}", version.0);

    pools.ping_all().await?;
    println!("所有 MySQL 连接健康检查通过");

    Ok(())
}
