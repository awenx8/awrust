use serde::Deserialize;

use super::Validate;
use crate::error::{ConfigResult, Error};

// ──────────────────────────────────────────────
// MySQL 配置
// ──────────────────────────────────────────────

/// 单个 MySQL 连接的配置。
///
/// 配置以连接串（`url`）为主，如 `mysql://user:pass@127.0.0.1:3306/db`；
/// `url` 为空时回退到逐字段配置（host/port/user/password/database）。
#[derive(Debug, Clone, Deserialize)]
pub struct MysqlConfig {
    /// 主连接串，如 `mysql://user:pass@host:3306/db`（url 模式）。
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_mysql_port")]
    pub port: u16,
    #[serde(default, alias = "username")]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub database: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_ssl_mode")]
    pub ssl_mode: String,
    #[serde(default)]
    pub disable_sql_mode: bool,
    #[serde(default = "default_acquire_timeout")]
    pub acquire_timeout: u32,
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u32,
}

impl Default for MysqlConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            host: String::new(),
            port: default_mysql_port(),
            user: String::new(),
            password: String::new(),
            database: String::new(),
            max_connections: default_max_connections(),
            ssl_mode: default_ssl_mode(),
            disable_sql_mode: false,
            acquire_timeout: default_acquire_timeout(),
            idle_timeout: default_idle_timeout(),
        }
    }
}

impl MysqlConfig {
    /// 返回主连接串；未设置（字段模式）时返回 `None`。
    pub fn url(&self) -> Option<&str> {
        (!self.url.is_empty()).then_some(self.url.as_str())
    }
}

fn default_mysql_port() -> u16 {
    3306
}
fn default_max_connections() -> u32 {
    10
}
fn default_acquire_timeout() -> u32 {
    5
}
fn default_idle_timeout() -> u32 {
    60
}
fn default_ssl_mode() -> String {
    "preferred".to_string()
}

impl Validate for MysqlConfig {
    fn validate(&self) -> ConfigResult<()> {
        if !self.url.is_empty() {
            // url 模式：连接串以 mysql:// 或 mariadb:// 开头即可。
            if !self.url.starts_with("mysql://") && !self.url.starts_with("mariadb://") {
                return Err(Error::ConfigValidation(format!(
                    "MySQL url 格式无效: `{}`，需以 `mysql://` 或 `mariadb://` 开头",
                    self.url
                )));
            }
        } else {
            // 字段模式：host / database / user 必填。
            if self.host.is_empty() {
                return Err(Error::ConfigValidation("MySQL host 不能为空".into()));
            }
            if self.database.is_empty() {
                return Err(Error::ConfigValidation("MySQL database 不能为空".into()));
            }
            if self.user.is_empty() {
                return Err(Error::ConfigValidation("MySQL user 不能为空".into()));
            }
            let valid_modes = [
                "disabled",
                "disable",
                "off",
                "preferred",
                "required",
                "require",
                "verify-ca",
                "verify_ca",
                "verify-identity",
                "verify_identity",
            ];
            if !valid_modes.contains(&self.ssl_mode.as_str()) {
                return Err(Error::ConfigValidation(format!(
                    "MySQL ssl_mode 无效: `{}`，可选: disabled, disable, off, preferred, required, require, verify-ca, verify-identity",
                    self.ssl_mode
                )));
            }
        }
        if self.port == 0 {
            return Err(Error::ConfigValidation("MySQL port 不能为 0".into()));
        }
        if self.max_connections == 0 {
            return Err(Error::ConfigValidation(
                "MySQL max_connections 不能为 0".into(),
            ));
        }
        if self.acquire_timeout == 0 {
            return Err(Error::ConfigValidation(
                "MySQL acquire_timeout 不能为 0".into(),
            ));
        }
        if self.idle_timeout == 0 {
            return Err(Error::ConfigValidation(
                "MySQL idle_timeout 不能为 0".into(),
            ));
        }
        Ok(())
    }
}

// ──────────────────────────────────────────────
// MySQL 子构建器
// ──────────────────────────────────────────────

/// MySQL 单连接构建器，提供链式 API。
pub struct MysqlConfigBuilder(pub(crate) MysqlConfig);

impl MysqlConfigBuilder {
    /// 设置主连接串，如 `mysql://user:pass@host:3306/db`。
    pub fn url(mut self, v: impl Into<String>) -> Self {
        self.0.url = v.into();
        self
    }
    pub fn host(mut self, v: impl Into<String>) -> Self {
        self.0.host = v.into();
        self
    }
    pub fn port(mut self, v: u16) -> Self {
        self.0.port = v;
        self
    }
    pub fn user(mut self, v: impl Into<String>) -> Self {
        self.0.user = v.into();
        self
    }
    pub fn password(mut self, v: impl Into<String>) -> Self {
        self.0.password = v.into();
        self
    }
    pub fn database(mut self, v: impl Into<String>) -> Self {
        self.0.database = v.into();
        self
    }
    pub fn max_connections(mut self, v: u32) -> Self {
        self.0.max_connections = v;
        self
    }
    pub fn ssl_mode(mut self, v: impl Into<String>) -> Self {
        self.0.ssl_mode = v.into();
        self
    }
    pub fn disable_sql_mode(mut self, v: bool) -> Self {
        self.0.disable_sql_mode = v;
        self
    }
    /// 设置连接超时时间（秒）
    pub fn acquire_timeout(mut self, v: u32) -> Self {
        self.0.acquire_timeout = v;
        self
    }
    /// 设置空闲连接回收时间（秒）
    pub fn idle_timeout(mut self, v: u32) -> Self {
        self.0.idle_timeout = v;
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
    fn validation_rejects_empty_host() {
        let result = ConfigBuilder::empty()
            .with_mysql("default", |m| {
                m.host("").user("u").password("p").database("db")
            })
            .build();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, crate::Error::ConfigValidation(ref msg) if msg.contains("host 不能为空"))
        );
    }

    #[test]
    fn url_accepts_valid_and_rejects_bad_scheme() {
        let ok = ConfigBuilder::empty()
            .with_mysql("default", |m| m.url("mysql://u:p@h:3306/db"))
            .build();
        assert!(ok.is_ok());

        let bad = ConfigBuilder::empty()
            .with_mysql("default", |m| m.url("http://u:p@h:3306/db"))
            .build();
        assert!(bad.is_err());
        let err = bad.unwrap_err();
        assert!(
            matches!(err, crate::Error::ConfigValidation(ref msg) if msg.contains("url 格式无效"))
        );
    }
}
