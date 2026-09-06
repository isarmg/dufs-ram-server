# Dufs Axum 迁移合同与验收记录

本文件区分已验证事实、目标合同和未完成工作，不作为已经迁移完成的声明。

## M0 基线

- Dufs：`3d4f486017aa1222fe9dec3428b910b90222b702`，0.50.2，开始时工作区干净。
- Foundation 工作区：`e000e9eb5285c5a396c54aac2f2c83ad37f9aef0`，0.6.1，开始时工作区干净。
- Dufs 实际 Foundation revision：`1e889d08fa69fcf2b5fffe45e8cc42b68218f4f1`，0.6.0。
- 工具：rustc 1.98.0 (88d9e12ae 2026-08-18)，cargo 1.98.0 (797e8a9bc 2026-08-05)，Node v26.7.0。
- 目标：x86_64-unknown-linux-gnu。

| 输入 | SHA-256 |
| --- | --- |
| Dufs Cargo.lock | `2a8763553276e0d1dc2b63c1045f8899e575a887519eb4490f7f17e13647d59c` |
| Dufs package-lock.json | `44208c8a4f75f324af9146021d46c5b0bc2c8efe787c3ab051514b1b2e10006a` |
| Dufs sarmg-product.toml | `e540dafd709d5e98a4bec536269cd0639ea1c859fd8559d7c310d8af61519e8b` |
| Dufs deploy/nginx-dufs.conf | `ed2d306c71445faf69bd6ef93fa9bd8959330a73de0ec2c7bf67f5a40f4f683d` |
| Foundation Cargo.lock | `60c197528c233f22a094ba12378e903e6e06a104de6c8e7041d6069d9497d1d8` |
| Foundation pnpm-lock.yaml | `8054b11556a3b2d25e80e1f10152b1c3424fa598f908499aff0c4b64ccc7abc8` |

基线命令：`npm run build:platform`；`cargo test --locked --target x86_64-unknown-linux-gnu --all-targets --all-features`。测试使用独立临时共享根、状态目录及动态端口，不访问真实实例状态。基线命令已通过。

## 明确的行为变化

| 项目 | 当前基线 | 迁移目标 |
| --- | --- | --- |
| 健康路径 | `/__dufs__/health`、`/__dufs__/ready` | `/healthz`、`/readyz`；旧路径不注册、不重定向 |
| 就绪认证 | 要求管理员 | 无认证，只有最小 `{ "ready": boolean }` |
| 管理认证 | Foundation Hyper Adapter | Foundation Axum Adapter，响应直接通过 |
| 请求 ID | 产品未统一 | Foundation 验证一次、生成一次、关联日志及响应 |
| GET/HEAD | 产品显式处理 | 登录、列表、操作查询仍 GET-only；文件 HEAD 保留上传查询分支 |
| 方法错误 | 产品处理；资源目录写入返回 404 | 显式方法拒绝，受保护接口先认证；公开静态资源只注册 GET/HEAD，其他方法 405 + Allow，无文件副作用 |
| 原始路径 | PathPolicy 一次解码 | 路由前 Service 验证并传递 RoutePath，不改写 URI |
| 新保留路径 | 可作为文件名 | 启动时只读检查冲突；明确拒绝，绝不删除、改名或静默隐藏 |

业务 JSON 预算维持 16 KiB，上传预检 2 MiB / 512 路径 / 256 KiB 路径字节 / 并发 4。PUT/PATCH 保持流式和实际字节限制，不用全局 TimeoutLayer 取消持久提交。静态管理员、内存 Session、rusqlite、原生 Web 和文件操作登记表不改模型。

## 状态与发布边界

新产品版本使用严格的当前 SchemaIdentity 和全新验收状态。不修改旧数据库 metadata，不把空状态库与旧暂存、待清理和未决操作拼接。无历史转换、原地升级或任意二进制回退承诺。真实实例保持停止且原数据不变。

正式依赖必须来自一个完整 Foundation Git revision；不得留下 sibling path、源码副本或双实现开关。Foundation 发布在前，Dufs 固定版本/revision、重新验证后发布。消费者矩阵只登记实际运行证据。

## 阶段跟踪

M0：上述全量 Rust 基线命令已通过，新增 `cargo test --locked --target x86_64-unknown-linux-gnu --test http_contract` 已通过（真实 socket，原始请求路径，10 个表驱动用例）；原有显式 ignored 用例尚未作为已验证项计入。M1–M9 的候选实现及全量 Rust 测试已通过；正式完整门禁、浏览器外观和发行 E2E 仍须完成。失败项不得标记 conforming。

## 当前实现与复验入口

Foundation 正式版本为 0.7.0，完整 revision 为 `77e7ad7af8e1bf62432bd6bdd8fa9aff54cb39d1`。
[CI](https://github.com/isarmg/sarmg-foundation-server/actions/runs/34037708626) 与
[正式发布](https://github.com/isarmg/sarmg-foundation-server/actions/runs/34037886724) 已通过。
该提交的 Rust 源码及 Cargo 清单/锁文件与完成消费者测试的 `99d5506c8146dfbe608e32441d97e51b924cdbb9` 完全一致；新增变化为原生 Web 导出及可选 React peer。

`release-tree.json` 的 SHA-256 为 `1844a4c53f2c2e92ce02501a228fcbca74e91d639911118b93456dd54667617f`。
Dufs 使用正式 tarball URL 与 npm integrity，不使用本地平台源码副本。

用户后续明确要求 Dufs 也采用 Foundation Web 外观：因此保留原生 ES modules，但统一六行登录卡片、顶部项目名/导航/图标、全宽文件表格、普通非连字字体和中英文主题。移除原样式、shell、字体快照，认证与业务模块不重写。构建把 Foundation CSS 的 SVG mask 发射成摘要命名的同源资源；登录仅增加 `img-src 'self'`，不开放 data:、内联脚本或外部来源。

| 手册用例 | 当前可执行证据 |
| --- | --- |
| R01、R03–R07 | `tests/auth.rs`、`tests/http_contract.rs`、`tests/browser_api/request_validation.rs`：唯一认证、页面与 API、重复安全头、CSRF、GET-only |
| R02 | `tests/library_api.rs::http_boundary_requires_a_real_socket_peer` |
| R08–R12 | `tests/http/resumable_upload.rs`、`tests/health.rs`、六组原始 HTTP fixture：上传 HEAD、未知内部路由、旧探针移除、公开最小响应、错误方法无副作用 |
| U01–U05 | `src/server/upload/tests.rs`、`tests/http/upload.rs`、`tests/http/resumable_upload.rs`：实际字节、EOF、offset、并发及空间水位 |
| U06–U09 | `src/server/router.rs`、`src/server/router/request.rs`、`src/server/tests.rs`、`tests/browser_api/jobs.rs`：提交边界、取消、日志、结果查询 |
| U10–U11 | `src/server/operation_registry/tests.rs`、`src/server/state_store/tests.rs`：指纹冲突、owner 隔离、已有结果 |
| U12–U13 | `src/server/storage.rs`、`src/server/upload/tests.rs`、`src/server/state_store/tests.rs`：同步/发布故障注入与重启恢复 |
| U14 | `tests/library_api.rs`、`tests/platform_namespace.rs`：只读启动冲突检查、物理路径别名不可绕过 |
| L01–L03 | `tests/bind.rs` 和 Foundation `listeners/transport` 的真实多地址 socket 测试 |
| L04–L06 | Foundation `transport/http1` 的慢头/写入期限/容量测试及 Dufs 下载分块、日志 Body 和 Range 测试 |
| L07–L08 | `tests/shutdown.rs`、`examples/runtime_shutdown_probe.rs`：SIGINT/SIGTERM、上传检查点、未完成提交不关闭状态 |
| L09、L11 | Foundation `tasks`、Dufs `listing/tests`、`tests/library_api.rs`：实际阻塞工作持有许可，停止后不能新登记 |
| L10、L12 | `scripts/check-release-runtime.sh`：真实 release fixture 非零退出与未完成计数；同一 release 二进制 SIGABRT 后重启核对上传状态 |

完整 Rust 回归命令为 `cargo test --locked --target x86_64-unknown-linux-gnu --all-targets --all-features --no-fail-fast`，0.51.0 / 正式 Foundation revision 已通过。
本地使用 `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0` 降低缓存占用，不更改产品 release panic 政策、断言、限额或覆盖率门槛。

基线十万目录项测试另行实际执行：旧二进制首屏 4.109828671 秒，候选首屏 4.858747356 秒，均通过既有 30 秒门槛。这是单次测量，不宣称性能提升；首次在并行重编译的资源压力下超时，空闲环境复验通过。普通全量测试中的显式 ignored 基准不等于自动执行。

发布前仍须核验 A1–A14 的最终完整门、覆盖率、浏览器、制品和消费者矩阵；该记录不以候选测试冒充未执行的正式发行验收。
