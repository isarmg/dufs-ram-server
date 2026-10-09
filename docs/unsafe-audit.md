# Rust 安全边界

本仓库的生产源码、构建脚本和 Rust 测试没有 `unsafe` 块、`unsafe fn` 或 `unsafe impl`。根 `Cargo.toml` 的 `unsafe_code = "forbid"` 作用于整个包，防止后续局部 `allow` 绕过这个边界。文件句柄、根目录访问、SQLite 上限与网络运行时通过安全 API 及固定 Foundation 输入使用。

这项检查只证明本仓库自有 Rust 代码的边界。Foundation、SQLx、rustix 及其他依赖的内部原生实现由各自源码和发行检查负责，不能把本仓库零 unsafe 写成完整依赖图零 unsafe。

目录保持单 Rust 包约定：根 `Cargo.toml` / `Cargo.lock`、`src/` 和 `tests/`；文件服务领域在 `src/server/`，CLI、认证与配置保留明确模块；浏览器资源在 `web/`，构建和检查入口在 `scripts/`，配置模板与主机部署资产分别在 `config/`、`deploy/`。本次没有改变包名、二进制名、协议或持久状态身份。

验证入口为 `cargo fmt --all -- --check`、Linux AMD64 的 `cargo clippy --locked --all-targets -- -D warnings` 和现有 Rust/浏览器质量门。macOS 静态检查不能代替 Linux 的 `openat2`、锁和最终发行物验收。

## 规范适用与本轮证据

| 条款 | 当前实现与验证边界 |
| --- | --- |
| 3、5 | Foundation 1.0.0 固定官方完整源码修订，manifest 与唯一 Cargo.lock 同步；官方输入策略需按该锁验证，Web 使用正式资产的真实 URL 与 SRI。 |
| 4.1、4.2 | 单 Rust 包、明确领域模块、web/config/deploy/scripts 职责；同步构建脚本、CI、文档和发行清单，不改历史标签/验收记录。 |
| 7、8、9 | 正常服务保持严格当前结构、显式 init 与只读配置校验；服务身份和私有数据属主限制保留。 |
| 用户 unsafe 审核要求、8、21 | 自有 unsafe 为零且整包 forbid；依赖的必要原生边界由固定 Foundation 和各依赖负责，不将静态检查写成 Linux 运行证据。 |
| 19–23、25 | Server 源码与当前文档不包含离线辅助工具协议；签名、Linux 运行、最终制品及设备证据分别验证。 |

本轮本地证据：Rust 格式检查及自有 unsafe 扫描；Linux 目标交叉静态验证按实际结果记录。macOS 本机没有执行 Linux 原生锁、生命周期或签名制品运行，正式 CI 的对应检查完成后才作为发行证据。

Node 文档格式/本地链接（55 文件）及独立性检查已通过；Rust/浏览器原生运行由实际 CI 执行。

正式 Foundation 1.0.0 的八个 Web tarball 已从真实 npm 缓存逐字节复核：SHA-512 与新 npm 锁一致，SHA-256 与官方 GitHub release 资产 digest 一致，包内版本均为 1.0.0；axe 浏览器验收依赖锁为 4.13.0。

真实正式 Web 输入的 Vite 生产构建、strict TypeScript、JavaScript 规则、Foundation/fonts 校验、文档/独立性检查已通过。49 项前端/脚本单测中 48 项本机通过，余下一项 release-archive Shell 测试受 macOS Bash 3 缺 `mapfile` 限制，保留 Linux CI 原测试，未降低门禁。

最终正式 Web 输出参与的 Linux AMD64 目标、全部 targets/features Clippy `-D warnings` 已通过；它是交叉静态验证，原生 Linux 行为及正式包结果仍由实际 CI/发行门确认。
