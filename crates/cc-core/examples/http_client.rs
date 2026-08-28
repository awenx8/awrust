//! HTTP 客户端示例：使用 HttpClient 进行微服务间调用。
//!
//! 演示功能：
//! - base_url 自动拼接
//! - 超时配置
//! - 默认请求头
//! - GET / POST / PUT / DELETE 请求
//! - JSON 请求体和响应
//!
//! 运行：`cargo run --example http_client`

use cc_core::HttpClient;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[tokio::main]
async fn main() -> cc_core::ConfigResult<()> {
    // 配置仅来自环境变量（或数据库，若启用 config-db）；此处仅读取 tracing 配置。
    let config = cc_core::ConfigBuilder::from_env()?.build()?;
    cc_core::tracing::init_tracing(&config.tracing)?;

    // 1. 创建 HTTP 客户端
    let client = HttpClient::builder()
        .base_url("https://jsonplaceholder.typicode.com")
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(5))
        .default_header("Accept", "application/json")
        .build()?;

    println!("base_url: {:?}", client.base_url());

    // 2. GET 请求 — 获取用户列表
    println!("\n--- GET /users ---");
    let resp = client.get("/users").send().await?;
    println!("状态码: {}", resp.status());
    let users: Vec<User> = resp.json().await?;
    println!("用户数量: {}", users.len());
    if let Some(first) = users.first() {
        println!("第一个用户: {} <{}>", first.name, first.email);
    }

    // 3. GET 请求 — 获取单个用户
    println!("\n--- GET /users/1 ---");
    let resp = client.get("/users/1").send().await?;
    let user: User = resp.json().await?;
    println!("用户: {:#?}", user);

    // 4. POST 请求 — 创建用户
    println!("\n--- POST /users ---");
    let new_user = CreateUser {
        name: "张三".into(),
        email: "zhangsan@example.com".into(),
    };
    let resp = client.post("/users").json(&new_user).send().await?;
    println!("状态码: {}", resp.status());
    let created: User = resp.json().await?;
    println!("创建的用户: {:#?}", created);

    // 5. PUT 请求 — 更新用户
    println!("\n--- PUT /users/1 ---");
    let updated_user = User {
        id: 1,
        name: "李四".into(),
        email: "lisi@example.com".into(),
    };
    let resp = client.put("/users/1").json(&updated_user).send().await?;
    println!("状态码: {}", resp.status());

    // 6. DELETE 请求 — 删除用户
    println!("\n--- DELETE /users/1 ---");
    let resp = client.delete("/users/1").send().await?;
    println!("状态码: {}", resp.status());

    // 7. 绝对 URL 不会被 base_url 影响
    println!("\n--- GET 绝对 URL ---");
    let resp = client.get("https://httpbin.org/status/200").send().await?;
    println!("状态码: {}", resp.status());

    println!("\n所有请求完成");
    Ok(())
}
