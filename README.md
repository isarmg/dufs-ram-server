# xczs

当前启动入口和初始化边界见 [服务命令](docs/cli.md)。部署须先显式 `init`，再 `run`；配置验证和状态查询失败会返回非零退出码。

Xczs `1.0.0` 是面向个人或受控局域网的轻量级 Web 文件管理器。一个 Rust 进程同时提供管理页面和文件服务，可浏览、搜索、上传、续传、下载、移动、重命名、删除和标记指定目录中的内容。

技术栈与其他 Server 项目统一：后端为 Rust 2024 / Tokio / Axum 和 SQLx / SQLite；前端为 React 19 / TypeScript / Vite，复用 xcss 的管理员认证、UI、设计令牌和字体。浏览器源码全部使用 `.ts`/`.tsx`，继承 `@xcss/web/web-toolchain/tsconfig.json` 的严格配置；构建先检查类型，再由 Vite 编译所有页面和文件业务模块，最后通过 `xcss::web_assets` 嵌入服务端。

当前正式运行目标仅为 Linux AMD64 GNU（`x86_64-unknown-linux-gnu`），并要求内核支持 `openat2`。生产入口应置于 HTTPS 反向代理之后；项目不提供匿名访问、普通用户角色、WebDAV、在线预览或移动端适配。

## 快速开始

使用 xcss 共同入口构建前端和服务端。正式模式要求干净的 Git 工作树：

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm run build:server:release -- --no-install
./target/x86_64-unknown-linux-gnu/release/xczs --version
```

先按[首次部署](docs/operations.md#1-首次部署)创建 `xczs` 服务账号和组，并准备该账号可访问的共享根与私有状态目录。生成管理员密码哈希，再从模板创建受保护的配置文件：

```sh
./target/x86_64-unknown-linux-gnu/release/xczs hash-password
sudo install -d -o root -g xczs -m 0750 /etc/xczs
sudo install -o root -g xczs -m 0640 config/xczs.json.example /etc/xczs/xczs.json
sudoedit /etc/xczs/xczs.json
```

至少设置 `serve_path`、`data_dir` 和 `auth`。`auth` 中应保留完整的 `username:$argon2id$...` 值：

```json
{
  "serve_path": "/srv/xczs",
  "data_dir": "/var/lib/xczs",
  "bind": [
    "127.0.0.1"
  ],
  "port": 5000,
  "auth": [
    "admin:$argon2id$REPLACE_WITH_THE_GENERATED_HASH"
  ]
}
```

启动前确保共享目录和状态目录可由服务账号访问，然后以前台方式验证：

```sh
sudo -u xczs ./target/x86_64-unknown-linux-gnu/release/xczs init --config /etc/xczs/xczs.json
sudo -u xczs ./target/x86_64-unknown-linux-gnu/release/xczs config validate --config /etc/xczs/xczs.json
sudo -u xczs ./target/x86_64-unknown-linux-gnu/release/xczs run --config /etc/xczs/xczs.json
```

默认示例只监听 `127.0.0.1:5000`。生产部署、反向代理、备份恢复和发行包验证见[运维文档](docs/operations.md)。

## 文件标签

登录后，顶部显示“文件、标签管理、服务状态”三个入口。文件管理的目录表直接显示“标签”列，二级菜单提供移动、下载、删除、重命名和标签图标：先选择操作，再点击文件或文件夹。标签模式只接受普通文件，菜单下方的标签行可连续点击多个标签添加关联，点击已选中的标签则移除关联；目录表同步展示当前文件的标签。文件夹不支持标签。搜索栏同时接受文件名与“全部/任一/排除”标签条件，筛选结果继续使用同一目录表和上传、下载、移动、重命名、删除操作。可创建、重命名、删除标签，通过二级菜单下方的标签行编辑文件标签；标签 API 保留批量操作、缺失记录与重新关联接口。服务状态页提供手动扫描和在线备份。文件下载沿用 Xczs 原有的受保护下载路径。

三个菜单共用一个 React 应用，通过 `#files`、`#tags`、`#status` 切换；标签筛选直接更新目录页的 `q`、`all`、`any`、`exclude` 查询条件。菜单导航及浏览器前进、后退无需重新加载文档，当前文件目录、列表、输入状态和上传任务保持有效。深链接使用当前目录地址和上述菜单 hash；未知菜单显示页面不存在，可通过顶部菜单重新选择，不改写书签地址。

索引与标签保存在 `data_dir/tags.db`，与共享文件根目录分离。服务启动后及按 `tag-scan-interval-seconds` 周期扫描，`max-tag-scan-entries` 限制单次扫描规模；手动扫描可随时触发。只有完整扫描成功才会标记缺失。外部或 Xczs 页面中的移动、改名、替换经扫描后按保守规则处理，可能需要通过标签 API 确认身份或手动重新关联，避免标签错误继承。标签备份写入 `data_dir/tag-backups/`；恢复时应停服并同时核对共享根与状态目录。详情见[运维文档](docs/operations.md)。

## 统一错误反馈

Web 与 API 使用一致的结构化错误和操作 ID；遇到写入结果未知时应先查询操作状态，不要直接重放。字段与恢复流程见[项目工作流程](docs/project-workflow.md)。

## 访问日志

访问日志会脱敏认证信息，并记录请求状态和操作 ID。生产文件权限、轮转、容量与停机刷新要求见[运维文档](docs/operations.md)。

## 开发验证

`npm run build:server -- --no-install` 生成未绑定源码的开发 binary，并验收其内嵌资源。需要前端热更新时，
运行时显式使用 `--development` 与绝对路径 `XCSS_DEV_WEB_DIR` 指向 `web/runtime-dist`；重新执行
`npm run build:platform` 即可更新资源，无需重新编译 Rust。源码绑定的正式 binary 拒绝目录覆盖。
目录模式使用共同 ETag 和重新验证缓存；正式资源路径绑定完整清单摘要，使用 immutable 缓存。

```sh
npm run build:server -- --no-install
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

当前发布版本：**1.0.0**。参见 [1.0.0 发布说明](docs/releases/1.0.0.md)。

公共支撑的职责、单体依赖、平台边界与验证方法见[公共支撑说明](docs/common-support.md)。
