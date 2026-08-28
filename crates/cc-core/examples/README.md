# cc-core 示例

## 配置

全部示例通过 `ConfigBuilder::auto()` 从 PostgreSQL 的 `app_config` 统一配置表读取配置。
设置数据库引导连接串环境变量后运行：

```bash
export APP_CONFIG_DATABASE_URL=postgres://postgres:secret@127.0.0.1:5432/configdb
```

注：`http_client` 仅读取 tracing 配置，使用 `ConfigBuilder::from_env()` 从环境变量构建，无需数据库。

## 示例列表

### config_from_db - 从数据库读取全部配置

```bash
cargo run --example config_from_db
```

### postgres_connect - PostgreSQL 连接（默认）

```bash
cargo run --example postgres_connect
```

### mysql_connect - MySQL 连接

启用 `mysql` feature 后运行：

```bash
cargo run --example mysql_connect --features mysql
```

### redis_connect - Redis 连接

```bash
cargo run --example redis_connect
```

### graceful_shutdown - 优雅关闭

监听 OS 信号（SIGTERM / SIGINT），按注册逆序执行关闭回调，支持 PostgreSQL 连接池和 Redis 管理器的一键关闭。

```bash
cargo run --example graceful_shutdown
```

### http_client - HTTP 客户端

基于 reqwest 的封装客户端，支持 base_url 自动拼接、超时配置、默认请求头。

```bash
cargo run --example http_client
```
