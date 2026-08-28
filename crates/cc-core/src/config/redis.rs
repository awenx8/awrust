use std::collections::HashMap;

use serde::Deserialize;

use super::Validate;
use super::split_env_field;
use crate::error::{ConfigResult, Error};

// ──────────────────────────────────────────────
// Redis 配置
// ──────────────────────────────────────────────

/// 单个 Redis 连接的配置。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RedisConfig {
    /// 形如 `redis://[:password@]host:port[/db]` 的连接串
    pub url: String,
}

impl Validate for RedisConfig {
    fn validate(&self) -> ConfigResult<()> {
        if self.url.is_empty() {
            return Err(Error::ConfigValidation("Redis url 不能为空".into()));
        }
        if !self.url.starts_with("redis://") && !self.url.starts_with("rediss://") {
            return Err(Error::ConfigValidation(format!(
                "Redis url 格式无效: `{}`，需以 `redis://` 或 `rediss://` 开头",
                self.url
            )));
        }
        Ok(())
    }
}

// ──────────────────────────────────────────────
// Redis 子构建器
// ──────────────────────────────────────────────

/// Redis 单连接构建器。
pub struct RedisConfigBuilder(pub(crate) RedisConfig);

impl Default for RedisConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RedisConfigBuilder {
    pub fn new() -> Self {
        Self(RedisConfig { url: String::new() })
    }

    pub fn url(mut self, v: impl Into<String>) -> Self {
        self.0.url = v.into();
        self
    }
}

// ──────────────────────────────────────────────
// 环境变量解析
// ──────────────────────────────────────────────

const REDIS_ENV_FIELDS: &[&str] = &["URL"];

pub(crate) fn collect_env_redis(
    prefix: &str,
    existing: &HashMap<String, RedisConfig>,
) -> ConfigResult<HashMap<String, RedisConfig>> {
    let mut result = HashMap::new();
    let pfx_upper = prefix.to_uppercase();
    let prefix_redis = format!("{pfx_upper}_REDIS_");

    for (key, val) in std::env::vars() {
        let upper = key.to_uppercase();
        let rest = match upper.strip_prefix(&prefix_redis) {
            Some(r) => r,
            None => continue,
        };

        let (name, field) = match split_env_field(rest, REDIS_ENV_FIELDS) {
            Some(v) => v,
            None => continue,
        };

        tracing::trace!(key = %key, name = %name, field = %field, "读取 Redis 环境变量");

        let entry = result
            .entry(name.clone())
            .or_insert_with(|| existing.get(&name).cloned().unwrap_or_default());

        if field == "URL" {
            entry.url = val;
        }
    }
    Ok(result)
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::ConfigBuilder;

    #[test]
    fn validation_rejects_bad_redis_url() {
        let result = ConfigBuilder::empty()
            .with_redis("default", |r| r.url("http://wrong"))
            .build();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::Error::ConfigValidation(ref msg) if msg.contains("redis://")));
    }
}
