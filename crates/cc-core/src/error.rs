//! 结构化错误类型，统一库边界错误处理。

/// cc-core 统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // ── 配置相关 ──
    /// 配置验证失败。
    #[error("配置验证失败: {0}")]
    ConfigValidation(String),

    /// 未配置数据库配置源的环境变量（`APP_CONFIG_DATABASE_URL` / `CC_CONFIG_DB_URL`）。
    #[error("未配置数据库配置连接串：请设置环境变量 APP_CONFIG_DATABASE_URL 或 CC_CONFIG_DB_URL")]
    ConfigDbUrlMissing,

    /// 从数据库加载配置失败。
    #[cfg(feature = "config-db")]
    #[error("从数据库加载配置失败（{url}）: {source}")]
    ConfigDatabase {
        url: String,
        #[source]
        source: sqlx::Error,
    },

    /// 配置值解析失败（如数值字段填了非数字）。
    #[cfg(feature = "config-db")]
    #[error("配置值解析失败: group=`{group}`, key=`{key}`, 期望 {expected}, 实际 `{value}`")]
    ConfigValueInvalid {
        group: String,
        key: String,
        expected: String,
        value: String,
    },

    // ── MySQL 相关 ──
    /// MySQL 连接建立失败。
    #[cfg(feature = "mysql")]
    #[error("连接 MySQL {target} 失败: {source}")]
    MysqlConnect {
        /// 连接目标：url 模式为脱敏后的连接串，字段模式为 `host:port`。
        target: String,
        #[source]
        source: sqlx::Error,
    },

    /// MySQL 连接池操作失败。
    #[cfg(feature = "mysql")]
    #[error("MySQL 连接池操作失败: {source}")]
    MysqlPool {
        #[source]
        source: sqlx::Error,
    },

    /// MySQL 连接名未找到。
    #[cfg(feature = "mysql")]
    #[error("未找到名为 `{name}` 的 MySQL 连接")]
    MysqlNotFound { name: String },

    /// MySQL 连接健康检查失败。
    #[cfg(feature = "mysql")]
    #[error("MySQL({name}) 健康检查失败: {message}")]
    MysqlHealthCheck {
        name: String,
        message: String,
        #[source]
        source: sqlx::Error,
    },

    // ── PostgreSQL 相关 ──
    /// PostgreSQL 连接建立失败。
    #[cfg(feature = "postgres")]
    #[error("连接 PostgreSQL {target} 失败: {source}")]
    PostgresConnect {
        /// 连接目标：url 模式为脱敏后的连接串，字段模式为 `host:port`。
        target: String,
        #[source]
        source: sqlx::Error,
    },

    /// PostgreSQL 连接池操作失败。
    #[cfg(feature = "postgres")]
    #[error("PostgreSQL 连接池操作失败: {source}")]
    PostgresPool {
        #[source]
        source: sqlx::Error,
    },

    /// PostgreSQL 连接名未找到。
    #[cfg(feature = "postgres")]
    #[error("未找到名为 `{name}` 的 PostgreSQL 连接")]
    PostgresNotFound { name: String },

    /// PostgreSQL 连接健康检查失败。
    #[cfg(feature = "postgres")]
    #[error("PostgreSQL({name}) 健康检查失败: {message}")]
    PostgresHealthCheck {
        name: String,
        message: String,
        #[source]
        source: sqlx::Error,
    },

    // ── Redis 相关 ──
    /// Redis 连接打开失败。
    #[cfg(feature = "redis")]
    #[error("打开 Redis({url}) 失败: {message}")]
    RedisOpen {
        url: String,
        message: String,
        #[source]
        source: redis::RedisError,
    },

    /// Redis 连接建立失败。
    #[cfg(feature = "redis")]
    #[error("连接 Redis({url}) 失败: {message}")]
    RedisConnect {
        url: String,
        message: String,
        #[source]
        source: redis::RedisError,
    },

    /// Redis 命令执行失败。
    #[cfg(feature = "redis")]
    #[error("Redis 命令执行失败: {0}")]
    RedisCommand(#[from] redis::RedisError),

    /// Redis 连接名未找到。
    #[cfg(feature = "redis")]
    #[error("未找到名为 `{name}` 的 Redis 连接")]
    RedisNotFound { name: String },

    /// Redis 连接健康检查失败。
    #[cfg(feature = "redis")]
    #[error("Redis({name}) 健康检查失败: {message}")]
    RedisHealthCheck {
        name: String,
        message: String,
        #[source]
        source: redis::RedisError,
    },

    // ── Tracing 相关 ──
    /// tracing subscriber 已初始化，不可重复调用。
    #[cfg(feature = "tracing-init")]
    #[error("tracing subscriber 已初始化，不可重复调用")]
    TracingAlreadyInit,

    /// 无效的日志级别。
    #[cfg(feature = "tracing-init")]
    #[error("无效的日志级别: {0}")]
    TracingInvalidLevel(String),

    // ── HTTP 相关 ──
    /// HTTP 客户端创建失败。
    #[cfg(feature = "http")]
    #[error("创建 HTTP 客户端失败: {message}")]
    HttpClientCreate { message: String },

    /// HTTP 请求失败。
    #[cfg(feature = "http")]
    #[error("HTTP 请求失败: {source}")]
    HttpRequest {
        #[source]
        source: reqwest::Error,
    },
}

/// 用于 Config::build() 的 Result 类型别名。
pub type ConfigResult<T> = std::result::Result<T, Error>;

/// 脱敏连接串：仅保留 scheme 与 `@` 之后的部分，隐藏用户与密码。
#[cfg(any(feature = "postgres", feature = "mysql", feature = "redis"))]
pub(crate) fn mask_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@')
        && let Some(scheme_end) = url.find("://")
    {
        let scheme = &url[..scheme_end + 3];
        let rest = &url[at_pos..];
        return format!("{scheme}****{rest}");
    }
    url.to_string()
}

#[cfg(feature = "http")]
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::HttpRequest { source: e }
    }
}
