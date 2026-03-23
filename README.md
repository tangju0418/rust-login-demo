# login_demo

一个基于 Rust + Axum + SQLite 的轻量服务 Demo,当前提供基础接口与静态页面,用于项目骨架验证和后续业务扩展。

## 目录结构

```text
.
├── src
│   ├── main.rs          # 程序入口,环境变量读取与服务启动
│   ├── web.rs           # 路由聚合与静态资源挂载
│   ├── infra
│   │   ├── mod.rs
│   │   └── db.rs        # SQLite 连接与迁移执行
│   └── rest
│       ├── mod.rs
│       ├── health.rs    # GET /health/get
│       └── echo.rs      # GET /echo/get
├── static
│   ├── index.html       # 前端页面入口
│   ├── app.js           # 前端调用 health/echo 接口
│   └── style.css        # 页面样式
├── migrations           # SQLite 迁移脚本目录(当前无业务表)
├── doc
│   └── README.md        # 文档入口
├── data                 # SQLite 数据文件目录(运行后创建/使用)
├── .env.example         # 环境变量模板
└── Cargo.toml
```

## 如何运行

1. 准备环境变量
   - 复制 `.env.example` 为 `.env`
   - 按需修改配置
2. 启动服务
   - 执行 `cargo run`
3. 访问页面与接口
   - 页面: [http://127.0.0.1:3000](http://127.0.0.1:3000) (如果你在 `.env` 改了端口,按实际端口访问)
   - 健康检查: [http://127.0.0.1:3000/health/get](http://127.0.0.1:3000/health/get)
   - 时间回显: [http://127.0.0.1:3000/echo/get](http://127.0.0.1:3000/echo/get)

## 环境变量

- `LISTEN_ADDR`
  - 说明: 服务监听地址
  - 默认: `0.0.0.0:3004`
- `DATABASE_URL`
  - 说明: SQLite 连接串
  - 默认: `sqlite:data/login_demo.db?mode=rwc`

## 当前核心架构

- `main` 启动层
  - 读取 `.env` 和环境变量
  - 保证数据库目录存在
  - 初始化数据库连接与迁移
  - 构建路由并启动 HTTP 服务
- `infra` 基础设施层
  - `infra/db.rs` 负责 DB 连接池创建和迁移执行
  - 对上层屏蔽底层连接细节
- `rest` 接口层
  - `health` 提供服务存活探测
  - `echo` 提供时间回显,用于快速验证服务链路
- `web` 路由层
  - 聚合 API 路由
  - 挂载 `static` 静态资源,提供前端页面

## 请求链路(简版)

1. 请求进入 Axum 路由(`web`)
2. 分发到对应处理器(`rest/health` 或 `rest/echo`)
3. 返回 JSON 响应给前端/调用方

## 说明

- 当前项目不包含具体业务模块,不包含业务表结构与预置业务数据。
- 现有日志采用单行格式,关键节点使用 `[Main]`、`[DB]`、`[Echo]` 前缀,便于链路排查。
