# Dufs HTTP 与运行时合同

本文描述当前 Dufs 的 HTTP 行为、Foundation 分工和验证入口。Rust 与 Web 依赖由
`Cargo.toml`、`Cargo.lock`、`package.json` 和 `package-lock.json` 固定到 Foundation 0.8.9。

## 请求与认证

| 入口或机制 | 当前行为 |
| --- | --- |
| `/healthz`、`/readyz` | 公开最小探针响应；详细状态仍需认证 |
| 管理员认证 | Foundation Axum Adapter 处理会话、Cookie、CSRF 和同源校验 |
| 登录与文件页面 | React 渲染页面结构，加载同源模块、共享 UI 和样式 |
| 请求 ID | Foundation 校验或生成 ID，供响应和日志关联 |
| GET/HEAD | 登录、列表和操作查询为 GET-only；文件 HEAD 包含上传状态查询分支 |
| 方法拒绝 | 受保护接口先认证；公开静态资源允许 GET/HEAD，其他方法返回 405 和 Allow |
| 原始路径 | 路由前验证并传递 RoutePath，保持原始 URI；路径策略只解码一次 |
| 保留路径 | 启动时只读检查冲突，发现冲突即拒绝启动 |

业务 JSON 上限为 16 KiB。上传预检上限为 2 MiB、512 条路径、256 KiB 路径字节，并发上限为 4。
PUT/PATCH 使用流式读取与实际字节限制。写入是否进入提交阶段决定超时、取消和恢复语义；
未知结果通过原操作 ID 查询，不能直接重放写入。详见[工作流程](project-workflow.md)。

## 代码边界

Foundation 负责通用认证、请求 ID、真实 socket peer、连接限制、信号和有界停机。
Dufs 负责共享根路径安全、文件流、列表、上传、持久操作和删除回收。静态管理员会话位于内存，
业务操作状态由 Dufs 的 SQLite actor 管理。

React 页面中的文件和上传控制器拥有各自 DOM 区域。主题更新保留正在运行的上传队列。
CSP 只允许同源静态资产，SVG 使用同源摘要路径。产品源码接受严格类型检查及 ESLint 浏览器安全检查。

## 验证入口

先构建 Web，生成 Rust 内嵌资源和 JavaScript 类型检查所需的 `web/dist/platform.js`：

```sh
npm run build:platform
npm run check:docs
npm run check:js
npm run check:types
npm run test:frontend:unit
cargo test --locked --target x86_64-unknown-linux-gnu --all-targets --all-features
```

| 行为 | 测试入口 |
| --- | --- |
| 认证、重复安全头、CSRF、GET-only | `tests/auth.rs`、`tests/http_contract.rs`、`tests/browser_api/request_validation.rs` |
| 真实 socket peer、启动路径冲突 | `tests/library_api.rs`、`tests/platform_namespace.rs` |
| 上传长度、offset、空间和恢复 | `tests/http/upload.rs`、`tests/http/resumable_upload.rs`、`src/server/upload/tests.rs` |
| 操作归属、指纹和持久结果 | `src/server/operation_registry/tests.rs`、`src/server/state_store/tests.rs` |
| 探针、多地址监听和停机 | `tests/health.rs`、`tests/bind.rs`、`tests/shutdown.rs` |
| 正式运行时和发行包 | `scripts/check-release-runtime.sh`、`scripts/check-formal-release-e2e.sh` |

测试使用独立临时共享根和状态目录。正式发布还要求候选包、完整源码提交、签名和验收结果一致；
具体流程见[运维文档](operations.md)。
