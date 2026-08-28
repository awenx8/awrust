# awrust

Rust 工作空间，包含 `cc-core` 核心公共库——提供「数据库 + 环境变量」配置系统 + PostgreSQL / MySQL / Redis
连接管理 + Tracing 日志初始化 + HTTP 客户端 + 优雅关闭。

## 项目结构

```text
awrust/
├── crates/
│   └── cc-core/              # 核心公共库
│       ├── src/
│       │   ├── lib.rs
│       │   ├── config/       # 数据库 + 环境变量配置系统
│       │   ├── postgres.rs   # PostgreSQL 连接池管理（默认）
│       │   ├── mysql.rs      # MySQL 连接池管理（可选）
│       │   ├── redis.rs      # Redis 连接管理
│       │   ├── tracing.rs    # Tracing 日志初始化
│       │   ├── http.rs       # HTTP 客户端（基于 reqwest）
│       │   └── shutdown.rs   # 优雅关闭管理器
│       └── examples/         # 使用示例
├── Cargo.toml                # 工作空间配置
└── justfile                  # 开发命令
```

## 快速开始

### 添加依赖

```bash
cargo add cc-core
```

默认启用 `postgres`、`redis`、`tracing-init`、`http`、`config-db` 特性；需要 MySQL 时
额外启用 `mysql` 特性：

```bash
cargo add cc-core --features mysql
```

### 配置来源（仅数据库与环境变量）

全部配置集中存储在 PostgreSQL 的 `app_config` 统一配置表，由 `ConfigBuilder::auto()`
（默认启用 `config-db` 特性）读取；数据库引导连接串与逐连接覆盖均来自环境变量：

```bash
export APP_CONFIG_DATABASE_URL=postgres://postgres:secret@127.0.0.1:5432/configdb
# 或 CC_CONFIG_DB_URL=postgres://postgres:secret@127.0.0.1:5432/configdb
```

```sql
-- app_config 统一配置表（全部配置来自这里）
INSERT INTO app_config (group_name, key, value) VALUES
    ('postgres', 'url', 'postgres://app:app_dev@127.0.0.1:5432/app'),
    ('redis',    'url', 'redis://:password@127.0.0.1:6379'),
    ('log',      'level', 'info'),
    ('log',      'format', 'pretty'),
    ('app',      'env', 'dev');
```

- 加载入口：`ConfigBuilder::auto()` 将数据库作为**唯一**结构化数据源，引导连接串取自
  `APP_CONFIG_DATABASE_URL` / `CC_CONFIG_DB_URL` 环境变量；未设置时返回错误。
- 环境变量作为最高优先级覆盖：`CC_MODE=<name>`、`CC_POSTGRES_<NAME>_URL`、
  `CC_MYSQL_<NAME>_URL`、`CC_REDIS_<NAME>_URL`、`CC_TRACING_LEVEL`、`CC_TRACING_FORMAT`。
- 仅从环境变量构建（无需数据库）：`ConfigBuilder::from_env()?.build()?`。
- PostgreSQL / MySQL 也兼容逐字段（host/port/user/password/database）回退写法，
  url 模式下非空的 host/user/password/database 字段仍可覆盖连接串中的对应部分。

### 功能特性

- **数据库驱动配置（config-db，默认启用）** — 数据库为唯一结构化数据源；环境变量提供引导连接串与最高优先级覆盖
- **PostgreSQL 连接池（默认）** — 多命名连接池管理，支持健康检查和优雅关闭
- **MySQL 连接池（可选）** — 多命名连接池管理，支持健康检查和优雅关闭
- **Redis 连接管理** — 多命名连接管理，支持自动重连和多路复用
- **Tracing 初始化** — 从配置读取日志级别和输出格式（json/pretty），一键初始化
- **HTTP 客户端** — 基于 reqwest 的薄封装，支持 base_url、超时、默认请求头
- **优雅关闭** — 注册回调式关闭管理器，内置 PostgreSQL / MySQL / Redis 便捷注册 + OS 信号监听

### 代码示例

| 示例                                                                 | 说明                            |
| -------------------------------------------------------------------- | ------------------------------- |
| [config_from_db.rs](crates/cc-core/examples/config_from_db.rs)       | 从数据库读取全部配置            |
| [postgres_connect.rs](crates/cc-core/examples/postgres_connect.rs)   | PostgreSQL 连接池管理（默认）   |
| [mysql_connect.rs](crates/cc-core/examples/mysql_connect.rs)         | MySQL 连接池管理                |
| [redis_connect.rs](crates/cc-core/examples/redis_connect.rs)         | Redis 连接管理                  |
| [graceful_shutdown.rs](crates/cc-core/examples/graceful_shutdown.rs) | 优雅关闭（信号监听 + 回调注册） |
| [http_client.rs](crates/cc-core/examples/http_client.rs)             | HTTP 客户端使用                 |

## 开发

```bash
just setup    # 初始化开发环境（安装依赖）
just dev      # 启动开发环境、跑通项目
just fmt      # 格式化代码（biome + rumdl + cargo fmt）
just lint     # 格式化 + 检查 + clippy
just test     # 运行测试
just examples # 运行所有示例
just verify   # fmt + lint + test + examples
```

## 许可证

MIT
