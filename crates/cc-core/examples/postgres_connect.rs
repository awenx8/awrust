use cc_core::{postgres::PostgresPools, ConfigBuilder, IntoConnectionName};

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
    // 从配置文件读取 PostgreSQL 引导连接，其余配置从 app_config 表加载
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
