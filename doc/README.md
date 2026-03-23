# service_demo

- 状态: active
- 日期: 2026-03-23

## 模块说明

- 后端: `axum` 提供 HTTP 接口与静态文件服务
- 存储: `sqlite` + `sqlx` 初始化连接与迁移管理(当前无业务表)
- 前端: `static` 目录下原生 HTML/CSS/JS

## 接口

- `GET /health/get`: 健康检查
- `GET /echo/get`: 返回服务端当前时间

## 启动

1. 复制 `.env.example` 为 `.env`
2. 执行 `cargo run`
3. 浏览器打开 [http://127.0.0.1:3000](http://127.0.0.1:3000)

## 环境变量

- `LISTEN_ADDR`: 默认 `0.0.0.0:3000`
- `DATABASE_URL`: 默认 `sqlite:data/login_demo.db?mode=rwc`
