//! 为 PostgreSQL / MySQL 连接池容器生成共享方法（`get` / `require` / `default` / `ping` /
//! `ping_all` / `stats` / `names` / `shutdown`），仅 `from_config` 与各数据库错误变体不同。

macro_rules! define_sql_pools {
    (
        $(#[$meta:meta])*
        $name:ident, $pool:ty,
        label = $label:literal,
        config_field = $field:ident,
        connect_fn = $connect:path,
        not_found = $not_found:ident,
        health_check = $health:ident,
    ) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $name {
            pools: std::collections::HashMap<String, $pool>,
        }

        /// 连接池统计信息。
        #[derive(Debug, Clone)]
        pub struct PoolStats {
            /// 当前活跃连接数
            pub active: usize,
            /// 当前空闲连接数
            pub idle: usize,
        }

        impl $name {
            /// 为配置里声明的每个 `$field` 连接建立连接池。
            pub async fn from_config(cfg: &crate::config::Config) -> ConfigResult<Self> {
                let mut pools = std::collections::HashMap::new();
                for (name, pc) in &cfg.$field {
                    tracing::info!(name = %name, concat!("初始化 ", $label, " 连接池"));
                    pools.insert(name.clone(), $connect(pc).await?);
                }
                tracing::info!(count = pools.len(), concat!("所有 ", $label, " 连接池初始化完成"));
                Ok(Self { pools })
            }

            /// 按名取连接池。
            pub fn get(&self, name: impl crate::config::IntoConnectionName) -> Option<&$pool> {
                self.pools.get(&name.into_name())
            }

            /// 按名取连接池，不存在时报错。
            pub fn require(
                &self,
                name: impl crate::config::IntoConnectionName,
            ) -> ConfigResult<&$pool> {
                let name = name.into_name();
                self.pools
                    .get(&name)
                    .ok_or_else(|| Error::$not_found { name })
            }

            /// 获取默认连接池（名字为 "default"）。
            pub fn default(&self) -> ConfigResult<&$pool> {
                self.require("default")
            }

            /// 健康检查：对指定连接执行 `SELECT 1`。
            pub async fn ping(
                &self,
                name: impl crate::config::IntoConnectionName,
            ) -> ConfigResult<()> {
                let name_str = name.into_name();
                let pool = self.require(&name_str)?;
                sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(pool)
                    .await
                    .map_err(|e| Error::$health {
                        name: name_str,
                        message: e.to_string(),
                        source: e,
                    })?;
                Ok(())
            }

            /// 健康检查：检查所有连接。
            pub async fn ping_all(&self) -> ConfigResult<()> {
                for name in self.pools.keys() {
                    self.ping(name.as_str()).await?;
                }
                Ok(())
            }

            /// 获取指定连接池的统计信息。
            pub fn stats(
                &self,
                name: impl crate::config::IntoConnectionName,
            ) -> Option<PoolStats> {
                let pool = self.pools.get(&name.into_name())?;
                let size = pool.size() as usize;
                let idle = pool.num_idle();
                Some(PoolStats {
                    active: size.saturating_sub(idle),
                    idle,
                })
            }

            /// 获取所有连接池名称。
            pub fn names(&self) -> impl Iterator<Item = &str> {
                self.pools.keys().map(String::as_str)
            }

            /// 关闭所有连接池。
            pub async fn shutdown(self) {
                tracing::info!(concat!("关闭所有 ", $label, " 连接池"));
                for (name, pool) in &self.pools {
                    tracing::debug!(name = %name, concat!("关闭 ", $label, " 连接池"));
                    pool.close().await;
                }
                tracing::info!(concat!("所有 ", $label, " 连接池已关闭"));
            }
        }
    };
}
