# Dufs 浏览器文件管理器

Dufs `0.51.15` 是面向个人或受控局域网的轻量级 Web 文件管理器。一个 Rust 进程同时提供管理页面和文件服务，可浏览、搜索、上传、续传、下载、移动、重命名及删除指定目录中的内容。

当前正式运行目标仅为 Linux AMD64 GNU（`x86_64-unknown-linux-gnu`），并要求内核支持 `openat2`。生产入口应置于 HTTPS 反向代理之后；项目不提供匿名访问、普通用户角色、WebDAV、在线预览或移动端适配。

## 快速开始

先构建前端并编译服务端：

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm run build:platform
cargo build --release --locked --target x86_64-unknown-linux-gnu
./target/x86_64-unknown-linux-gnu/release/dufs --version
```

生成管理员密码哈希，再从模板创建受保护的配置文件：

```sh
./target/x86_64-unknown-linux-gnu/release/dufs hash-password
sudo install -d -m 0750 /etc/dufs
sudo install -m 0600 config/dufs.yaml.example /etc/dufs/dufs.yaml
sudoedit /etc/dufs/dufs.yaml
```

至少设置 `serve-path`、`state-dir` 和 `auth`。`auth` 中应保留完整的 `username:$argon2id$...` 值：

```yaml
serve-path: /srv/dufs
state-dir: /var/lib/dufs
bind:
  - 127.0.0.1
port: 5000
auth:
  - 'admin:$argon2id$REPLACE_WITH_THE_GENERATED_HASH'
```

启动前确保共享目录和状态目录可由服务账号访问，然后以前台方式验证：

```sh
sudo ./target/x86_64-unknown-linux-gnu/release/dufs --config /etc/dufs/dufs.yaml
```

默认示例只监听 `127.0.0.1:5000`。生产部署、反向代理、备份恢复和发行包验证见[运维文档](docs/operations.md)。

## 统一错误反馈

Web 与 API 使用一致的结构化错误和操作 ID；遇到写入结果未知时应先查询操作状态，不要直接重放。字段与恢复流程见[项目工作流程](docs/project-workflow.md)。

## 访问日志

访问日志会脱敏认证信息，并记录请求状态和操作 ID。生产文件权限、轮转、容量与停机刷新要求见[运维文档](docs/operations.md)。

## 开发验证

```sh
npm run build:platform
cargo fmt --all -- --check
cargo clippy --locked --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
cargo test --locked --target x86_64-unknown-linux-gnu
npm run check:docs
npm run check:js
npm run check:types
npm run test:frontend:unit
```

## 文档

- [文档总览](docs/README.md)
- [初学者指南](docs/beginner-guide/README.md)
- [项目工作流程](docs/project-workflow.md)
- [功能范围与取舍](docs/feature-inventory-and-tradeoffs.md)
- [部署与运维](docs/operations.md)

代码采用 [Apache License 2.0](LICENSE-APACHE)。
