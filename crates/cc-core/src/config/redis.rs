use serde::Deserialize;

use super::Validate;
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
