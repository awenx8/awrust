//! 从 PostgreSQL 的 `app_config` 统一配置表读取全部配置。
//!
//! # 配置表结构
//!
//! ```sql
//! CREATE TABLE app_config (
//!     group_name   TEXT    NOT NULL DEFAULT 'default',
//!     key          TEXT    NOT NULL,
//!     value        TEXT    NOT NULL,
//!     description  TEXT    NOT NULL DEFAULT '',
//!     PRIMARY KEY (group_name, key)
//! );
//! ```
//!
//! # 分组 → cc-core 配置 映射约定
//!
//! | DB 分组       | cc-core 目标            | 说明                                          |
//! | ------------ | ----------------------- | --------------------------------------------- |
//! | `app`        | `mode`（`env` 键）       | 运行模式                                       |
//! | `log`        | `tracing`（level/format）| 优先读取；`tracing` 分组亦可          |
//! | `redis`      | `redis`                 | 键 `url` → 连接名 `default`；`<名>.url` → 命名 |
//! | `postgres`   | `postgres`              | 键 `url` → 连接名 `default`；`<名>.url` / `<名>.<字段>` → 命名 |
//! | `mysql`      | `mysql`                 | 键 `url` → 连接名 `default`；`<名>.url` / `<名>.<字段>` → 命名 |
//! | 其它分组      | `store`                 | 全部保留为强类型键值，供程序化读取              |
//!
//! 仅当启用 `config-db` feature 时编译。

use std::collections::HashMap;

use sqlx::Row;

use super::super::error::{ConfigResult, Error};
use super::{Config, ConfigStore, MysqlConfig, PostgresConfig, RedisConfig, TracingConfig};

/// 数据库配置连接串的环境变量（按优先级尝试）。
pub const CONFIG_DB_URL_ENV: &str = "APP_CONFIG_DATABASE_URL";
pub const CONFIG_DB_URL_ENV_LEGACY: &str = "CC_CONFIG_DB_URL";

/// 读取数据库配置连接串环境变量；未设置返回 `None`。
pub fn config_database_url() -> Option<String> {
    std::env::var(CONFIG_DB_URL_ENV)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::env::var(CONFIG_DB_URL_ENV_LEGACY)
                .ok()
                .filter(|v| !v.is_empty())
        })
}

/// 从 PostgreSQL 的 `app_config` 表读取全部配置行，组装为 `ConfigStore`。
pub async fn load_config_store(url: &str) -> ConfigResult<ConfigStore> {
    ::tracing::info!(url = %url, "从数据库加载统一配置");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(url)
        .await
        .map_err(|e| Error::ConfigDatabase {
            url: url.to_string(),
            source: e,
        })?;

    let rows =
        sqlx::query("SELECT group_name, key, value FROM app_config ORDER BY group_name, key")
            .fetch_all(&pool)
            .await
            .map_err(|e| Error::ConfigDatabase {
                url: url.to_string(),
                source: e,
            })?;
    pool.close().await;

    let mut store: ConfigStore = HashMap::new();
    for row in &rows {
        let group: String = row
            .try_get("group_name")
            .map_err(|e| Error::ConfigDatabase {
                url: url.to_string(),
                source: e,
            })?;
        let key: String = row.try_get("key").map_err(|e| Error::ConfigDatabase {
            url: url.to_string(),
            source: e,
        })?;
        let value: String = row.try_get("value").map_err(|e| Error::ConfigDatabase {
            url: url.to_string(),
            source: e,
        })?;

        store.entry(group).or_default().insert(key, value);
    }

    ::tracing::info!(groups = store.len(), "数据库配置加载完成");
    Ok(store)
}

/// 将 `ConfigStore` 映射为 cc-core 的 `Config`（同时保留全部分组键值）。
pub fn build_config(store: ConfigStore, mode: Option<String>) -> ConfigResult<Config> {
    let mode = mode.or_else(|| store.get("app").and_then(|g| g.get("env")).cloned());

    let tracing = build_tracing(&store)?;
    let redis = build_redis(&store)?;
    let postgres = build_postgres(&store)?;
    let mysql = build_mysql(&store)?;

    Ok(Config {
        mode,
        postgres,
        mysql,
        redis,
        tracing,
        store,
    })
}

fn build_tracing(store: &ConfigStore) -> ConfigResult<TracingConfig> {
    // 优先 `log` 分组，回退 `tracing` 分组。
    let group = store.get("log").or_else(|| store.get("tracing"));
    let mut cfg = TracingConfig::default();
    if let Some(g) = group {
        if let Some(v) = g.get("level") {
            cfg.level = v.clone();
        }
        if let Some(v) = g.get("format") {
            cfg.format = v.clone();
        }
    }
    Ok(cfg)
}

fn build_redis(store: &ConfigStore) -> ConfigResult<HashMap<String, RedisConfig>> {
    let mut out = HashMap::new();
    if let Some(group) = store.get("redis") {
        for (key, value) in group {
            let url: &str = value;
            // 键 `url` → 连接名 default；`<名>.url` → 命名连接。
            let name = match key.split_once('.') {
                Some((name, field)) if field.eq_ignore_ascii_case("url") => name.to_string(),
                _ if key.eq_ignore_ascii_case("url") => "default".to_string(),
                _ => continue,
            };
            out.insert(
                name,
                RedisConfig {
                    url: url.to_string(),
                },
            );
        }
    }
    Ok(out)
}

/// 取分组内某个键的整型值；值存在但解析失败时返回错误。
fn int_of(group: &HashMap<String, String>, gname: &str, key: &str) -> ConfigResult<Option<i64>> {
    match group.get(key) {
        Some(v) => v
            .trim()
            .parse::<i64>()
            .map(Some)
            .map_err(|_| Error::ConfigValueInvalid {
                group: gname.to_string(),
                key: key.to_string(),
                expected: "int".into(),
                value: v.clone(),
            }),
        None => Ok(None),
    }
}

/// 取分组内某个键的布尔值；值存在但解析失败时返回错误。
fn bool_of(group: &HashMap<String, String>, gname: &str, key: &str) -> ConfigResult<Option<bool>> {
    match group.get(key) {
        Some(v) => super::parse_bool(v)
            .map(Some)
            .ok_or_else(|| Error::ConfigValueInvalid {
                group: gname.to_string(),
                key: key.to_string(),
                expected: "bool".into(),
                value: v.clone(),
            }),
        None => Ok(None),
    }
}

/// 将 `app_config` 中某分组的扁平键值按 `name.field` 拆分成按连接名归组的字段表。
///
/// 键格式：`url` → 连接名 `default`；`<连接名>.<字段>`（如 `default.host`）→ 命名连接。
fn named_fields(store: &ConfigStore, group: &str) -> HashMap<String, HashMap<String, String>> {
    let mut by_name: HashMap<String, HashMap<String, String>> = HashMap::new();
    if let Some(group_map) = store.get(group) {
        for (key, value) in group_map {
            let (name, field) = match key.split_once('.') {
                Some((name, field)) => (name.to_string(), field.to_string()),
                None => ("default".to_string(), key.clone()),
            };
            by_name
                .entry(name)
                .or_default()
                .insert(field, value.clone());
        }
    }
    by_name
}

fn build_postgres(store: &ConfigStore) -> ConfigResult<HashMap<String, PostgresConfig>> {
    let by_name = named_fields(store, "postgres");

    let mut out = HashMap::new();
    for (name, fields) in &by_name {
        let mut cfg = PostgresConfig::default();
        let g = "postgres";
        if let Some(v) = fields.get("url") {
            cfg.url = v.clone();
        }
        if let Some(v) = fields.get("host") {
            cfg.host = v.clone();
        }
        if let Some(v) = fields.get("user") {
            cfg.user = v.clone();
        }
        if let Some(v) = fields.get("password") {
            cfg.password = v.clone();
        }
        if let Some(v) = fields.get("database") {
            cfg.database = v.clone();
        }
        if let Some(v) = fields.get("ssl_mode") {
            cfg.ssl_mode = v.clone();
        }
        if let Some(p) = int_of(fields, g, "port")? {
            cfg.port = p as u16;
        }
        if let Some(p) = int_of(fields, g, "max_connections")? {
            cfg.max_connections = p as u32;
        }
        if let Some(p) = int_of(fields, g, "acquire_timeout")? {
            cfg.acquire_timeout = p as u32;
        }
        if let Some(p) = int_of(fields, g, "idle_timeout")? {
            cfg.idle_timeout = p as u32;
        }
        out.insert(name.clone(), cfg);
    }
    Ok(out)
}

fn build_mysql(store: &ConfigStore) -> ConfigResult<HashMap<String, MysqlConfig>> {
    let by_name = named_fields(store, "mysql");

    let mut out = HashMap::new();
    for (name, fields) in &by_name {
        let mut cfg = MysqlConfig::default();
        let g = "mysql";
        if let Some(v) = fields.get("url") {
            cfg.url = v.clone();
        }
        if let Some(v) = fields.get("host") {
            cfg.host = v.clone();
        }
        if let Some(v) = fields.get("user") {
            cfg.user = v.clone();
        }
        if let Some(v) = fields.get("password") {
            cfg.password = v.clone();
        }
        if let Some(v) = fields.get("database") {
            cfg.database = v.clone();
        }
        if let Some(v) = fields.get("ssl_mode") {
            cfg.ssl_mode = v.clone();
        }
        if let Some(p) = int_of(fields, g, "port")? {
            cfg.port = p as u16;
        }
        if let Some(p) = int_of(fields, g, "max_connections")? {
            cfg.max_connections = p as u32;
        }
        if let Some(p) = int_of(fields, g, "acquire_timeout")? {
            cfg.acquire_timeout = p as u32;
        }
        if let Some(p) = int_of(fields, g, "idle_timeout")? {
            cfg.idle_timeout = p as u32;
        }
        if let Some(b) = bool_of(fields, g, "disable_sql_mode")? {
            cfg.disable_sql_mode = b;
        }
        out.insert(name.clone(), cfg);
    }
    Ok(out)
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_from_store() {
        let mut store: ConfigStore = HashMap::new();

        store.insert("app".into(), HashMap::from([("env".into(), "dev".into())]));
        store.insert(
            "log".into(),
            HashMap::from([
                ("level".into(), "debug".into()),
                ("format".into(), "json".into()),
            ]),
        );
        store.insert(
            "redis".into(),
            HashMap::from([("url".into(), "redis://127.0.0.1:6379".into())]),
        );
        // postgres 使用连接串为主（与配置种子一致）：`url` → 默认连接。
        store.insert(
            "postgres".into(),
            HashMap::from([(
                "url".into(),
                "postgres://postgres:pg-pw@pg-host:5432/pgdb".into(),
            )]),
        );

        // mysql 使用逐字段配置（兼容字段模式）。
        let mut mysql_fields = HashMap::new();
        mysql_fields.insert("default.host".into(), "db-host".into());
        mysql_fields.insert("default.port".into(), "3306".into());
        mysql_fields.insert("default.user".into(), "root".into());
        mysql_fields.insert("default.password".into(), "pw".into());
        mysql_fields.insert("default.database".into(), "mydb".into());
        store.insert("mysql".into(), mysql_fields);

        let cfg = build_config(store, None).unwrap();
        assert_eq!(cfg.mode(), Some("dev"));
        assert_eq!(cfg.tracing.level, "debug");
        assert_eq!(cfg.tracing.format, "json");
        assert_eq!(cfg.redis("default").unwrap().url, "redis://127.0.0.1:6379");
        let p = cfg.postgres("default").unwrap();
        assert_eq!(p.url(), Some("postgres://postgres:pg-pw@pg-host:5432/pgdb"));
        let m = cfg.mysql("default").unwrap();
        assert_eq!(m.host, "db-host");
        assert_eq!(m.port, 3306);
        assert_eq!(m.database, "mydb");
    }

    #[test]
    fn build_named_redis() {
        let mut store: ConfigStore = HashMap::new();
        let mut redis = HashMap::new();
        redis.insert("cache.url".into(), "redis://127.0.0.1:6380".into());
        store.insert("redis".into(), redis);
        let cfg = build_config(store, None).unwrap();
        assert_eq!(cfg.redis("cache").unwrap().url, "redis://127.0.0.1:6380");
        assert!(cfg.redis("default").is_none());
    }

    #[test]
    fn build_named_postgres_url() {
        let mut store: ConfigStore = HashMap::new();
        store.insert(
            "postgres".into(),
            HashMap::from([("warehouse.url".into(), "postgres://u:p@wh:5432/wdb".into())]),
        );
        let cfg = build_config(store, None).unwrap();
        assert_eq!(
            cfg.postgres("warehouse").unwrap().url(),
            Some("postgres://u:p@wh:5432/wdb")
        );
        assert!(cfg.postgres("default").is_none());
    }
}
