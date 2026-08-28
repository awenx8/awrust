use cc_core::{ConfigBuilder, IntoConnectionName, postgres::PostgresPools};

enum PostgresName {
    Default,
}

impl IntoConnectionName for PostgresName {
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

    let pools = PostgresPools::from_config(&config).await?;
    let pool = pools.require(PostgresName::Default)?;

    let version: (String,) = sqlx::query_as("SELECT version()")
        .fetch_one(pool)
        .await
        .map_err(|e| cc_core::Error::PostgresPool { source: e })?;
    println!("PostgreSQL: {}", version.0);

    pools.ping_all().await?;
    println!("所有 PostgreSQL 连接健康检查通过");

    Ok(())
}
