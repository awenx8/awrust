use serde::Deserialize;

use super::Validate;
use crate::error::{ConfigResult, Error};

// ──────────────────────────────────────────────
// Tracing 配置
// ──────────────────────────────────────────────

/// 合法的日志级别列表。
const VALID_LEVELS: &[&str] = &["trace", "debug", "info", "warn", "error"];

/// 合法的输出格式列表。
const VALID_FORMATS: &[&str] = &["pretty", "json"];

fn default_level() -> String {
    "info".to_string()
}

fn default_format() -> String {
    "pretty".to_string()
}

/// Tracing 日志配置。
#[derive(Debug, Clone, Deserialize)]
pub struct TracingConfig {
    /// 日志级别：trace / debug / info / warn / error（默认 "info"）
    #[serde(default = "default_level")]
    pub level: String,
    /// 输出格式：pretty / json（默认 "pretty"）
    #[serde(default = "default_format")]
    pub format: String,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            level: default_level(),
            format: default_format(),
        }
    }
}

impl Validate for TracingConfig {
    fn validate(&self) -> ConfigResult<()> {
        let level_lower = self.level.to_ascii_lowercase();
        if !VALID_LEVELS.contains(&level_lower.as_str()) {
            return Err(Error::ConfigValidation(format!(
                "tracing level 无效: `{}`，可选: {}",
                self.level,
                VALID_LEVELS.join(", ")
            )));
        }
        let format_lower = self.format.to_ascii_lowercase();
        if !VALID_FORMATS.contains(&format_lower.as_str()) {
            return Err(Error::ConfigValidation(format!(
                "tracing format 无效: `{}`，可选: {}",
                self.format,
                VALID_FORMATS.join(", ")
            )));
        }
        Ok(())
    }
}

// ──────────────────────────────────────────────
// Tracing 子构建器
// ──────────────────────────────────────────────

/// Tracing 配置构建器，提供链式 API。
pub struct TracingConfigBuilder(pub(crate) TracingConfig);

impl Default for TracingConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TracingConfigBuilder {
    pub fn new() -> Self {
        Self(TracingConfig::default())
    }

    /// 设置日志级别（trace / debug / info / warn / error）。
    pub fn level(mut self, v: impl Into<String>) -> Self {
        self.0.level = v.into();
        self
    }

    /// 设置输出格式（pretty / json）。
    pub fn format(mut self, v: impl Into<String>) -> Self {
        self.0.format = v.into();
        self
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let cfg = TracingConfig::default();
        assert_eq!(cfg.level, "info");
        assert_eq!(cfg.format, "pretty");
    }

    #[test]
    fn validate_accepts_valid() {
        let cfg = TracingConfig {
            level: "debug".into(),
            format: "json".into(),
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_rejects_bad_level() {
        let cfg = TracingConfig {
            level: "verbose".into(),
            format: "pretty".into(),
        };
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, Error::ConfigValidation(ref msg) if msg.contains("level 无效")));
    }

    #[test]
    fn validate_rejects_bad_format() {
        let cfg = TracingConfig {
            level: "info".into(),
            format: "xml".into(),
        };
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, Error::ConfigValidation(ref msg) if msg.contains("format 无效")));
    }

    #[test]
    fn builder_works() {
        let cfg = TracingConfigBuilder::new().level("warn").format("json").0;
        assert_eq!(cfg.level, "warn");
        assert_eq!(cfg.format, "json");
    }
}
