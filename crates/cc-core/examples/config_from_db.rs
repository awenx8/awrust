//! 从数据库读取全部配置（`app_config` 统一配置表）。
//!
//! 引导连接取环境变量 `APP_CONFIG_DATABASE_URL` / `CC_CONFIG_DB_URL`；
//! 未设置时返回错误：
//!
//! ```sh
//! APP_CONFIG_DATABASE_URL=postgres://postgres:secret@127.0.0.1:5432/configdb \
//! cargo run -p cc-core --features config-db --example config_from_db
//! ```

#[cfg(feature = "config-db")]
#[tokio::main]
async fn main() -> cc_core::ConfigResult<()> {
    // 加载 `.env`（若存在）中的引导连接串等环境变量。
    dotenvy::dotenv().ok();

    // 自动选择来源：检测到 APP_CONFIG_DATABASE_URL 时从数据库读取全部配置。
    let config = match cc_core::ConfigBuilder::auto().await {
        Ok(config) => config,
        Err(cc_core::Error::ConfigDbUrlMissing) => {
            eprintln!(
                "未设置引导连接串，请通过环境变量指定承载 app_config 表的数据库：\n\
                 \x20\x20APP_CONFIG_DATABASE_URL=postgres://postgres:secret@127.0.0.1:5432/configdb \\\n\
                 \x20\x20cargo run -p cc-core --features config-db --example config_from_db"
            );
            std::process::exit(2);
        }
        Err(e) => {
            eprintln!("加载配置失败: {e}");
            std::process::exit(1);
        }
    };

    cc_core::tracing::init_tracing(&config.tracing)?;

    println!("运行模式: {:?}", config.mode());

    if let Some(postgres) = config.postgres("default") {
        println!("PostgreSQL: {}", postgres.url().unwrap_or("(未设置)"));
    }
    if let Some(mysql) = config.mysql("default") {
        println!("MySQL: {}", mysql.url().unwrap_or("(未设置)"));
    }
    if let Some(redis) = config.redis("default") {
        println!("Redis: {}", redis.url);
    }

    // 读取其它分组的原始配置值。
    if let Some(env) = config.get_value("app", "env") {
        println!("app.env = {env}");
    }
    if let Some(maintenance) = config.get_bool("feature", "maintenance_mode") {
        println!("feature.maintenance_mode = {maintenance}");
    }

    Ok(())
}

#[cfg(not(feature = "config-db"))]
fn main() {
    eprintln!("请使用 `--features config-db` 启用数据库配置加载");
}
