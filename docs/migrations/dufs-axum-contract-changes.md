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

基线命令：`npm run build:platform`；`cargo test --locked --target x86_64-unknown-linux-gnu --all-targets --all-features`。测试使用独立临时共享根、状态目录及动态端口，不访问真实实例状态。执行结果在完成后补录。

## 明确的行为变化

| 项目 | 当前基线 | 迁移目标 |
| --- | --- | --- |
| 健康路径 | `/__dufs__/health`、`/__dufs__/ready` | `/healthz`、`/readyz`；旧路径不注册、不重定向 |
| 就绪认证 | 要求管理员 | 无认证，只有最小 `{ "ready": boolean }` |
| 管理认证 | Foundation Hyper Adapter | Foundation Axum Adapter，响应直接通过 |
| 请求 ID | 产品未统一 | Foundation 验证一次、生成一次、关联日志及响应 |
| GET/HEAD | 产品显式处理 | 登录、列表、操作查询仍 GET-only；文件 HEAD 保留上传查询分支 |
| 方法错误 | 产品处理 | 显式方法拒绝，受保护接口先认证，无文件副作用 |
| 原始路径 | PathPolicy 一次解码 | 路由前 Service 验证并传递 RoutePath，不改写 URI |
| 新保留路径 | 可作为文件名 | 启动时只读检查冲突；明确拒绝，绝不删除、改名或静默隐藏 |

业务 JSON 预算维持 16 KiB，上传预检 2 MiB / 512 路径 / 256 KiB 路径字节 / 并发 4。PUT/PATCH 保持流式和实际字节限制，不用全局 TimeoutLayer 取消持久提交。静态管理员、内存 Session、rusqlite、原生 Web 和文件操作登记表不改模型。

## 状态与发布边界

新产品版本使用严格的当前 SchemaIdentity 和全新验收状态。不修改旧数据库 metadata，不把空状态库与旧暂存、待清理和未决操作拼接。无历史转换、原地升级或任意二进制回退承诺。真实实例保持停止且原数据不变。

正式依赖必须来自一个完整 Foundation Git revision；不得留下 sibling path、源码副本或双实现开关。Foundation 发布在前，Dufs 固定版本/revision、重新验证后发布。消费者矩阵只登记实际运行证据。

## 阶段跟踪

M0：上述全量 Rust 基线命令已通过，新增 `cargo test --locked --target x86_64-unknown-linux-gnu --test http_contract` 已通过（真实 socket，原始请求路径，10 个表驱动用例）；原有显式 ignored 用例尚未作为已验证项计入。M1 平台能力、M2 Body、M3 路由、M4 认证、M5 路径、M6 上传、M7 下载日志、M8 文件操作、M9 生命周期、M10 发布均待验收。失败项不得标记 conforming。
