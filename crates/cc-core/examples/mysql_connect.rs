use cc_core::{ConfigBuilder, IntoConnectionName, mysql::MysqlPools};

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
    // 从环境变量读取引导连接串（APP_CONFIG_DATABASE_URL / CC_CONFIG_DB_URL），连接其 app_config 表加载全部配置
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
