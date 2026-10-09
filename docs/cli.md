# 当前服务命令

所有命令使用同一份当前 JSON 配置：`xczs --config /absolute/config.json --data-dir /absolute/data ...`。配置来源优先级为命令行、明确映射的环境变量、文件、默认值。未知字段、重复字段、错误类型和坏的低优先级配置都会被拒绝；诊断不会打印秘密值。`config validate --json` 中 `sources` 是字段的来源，`schema_identity` 是校验过的当前结构身份，`state_paths` 列出全部私有持久状态的绝对路径。

```sh
xczs --help
xczs --version
xczs init --config /absolute/config.json --data-dir /absolute/data
xczs config validate --config /absolute/config.json --data-dir /absolute/data --json
xczs run --config /absolute/config.json --data-dir /absolute/data
xczs status --config /absolute/config.json --data-dir /absolute/data --json
```

`init` 是创建数据库和首个管理员的唯一部署入口，只接受尚未初始化的私有空目录。配置和管理员凭据必须先有效；已有数据不会被覆盖。数据目录应属于服务账号、权限为 `0700`，私有文件权限为 `0600`。Xczs 发行包沿用其编译绑定的发行身份校验，运行命令是 `run`。

`run` 只接受完整的当前数据；缺失数据库、管理员、结构漂移或身份不符均失败，不隐式初始化或重置账号。运行及写入维护命令使用同一个数据目录维护锁；整个运行期间持有实例锁。`.xcss-maintenance-pending.json` 存在时拒绝运行、初始化或写入维护；只读检查仍可运行。

`config validate` 用私有 SQLite 快照检验状态库和标签库，并只读验证静态管理员文件；原库、WAL、SHM 和业务文件保持不变。`status` 查询当前监听地址的 `/readyz`，核对服务身份和真实业务就绪；端口占用、其他服务、连接失败或未就绪均返回非零退出码。`--json` 输出单个机器记录；失败返回稳定 `code/message/details` 错误记录。帮助和版本查询不要求初始化。

共享配置、命令、快照与日志均固定到同一 Foundation Git 完整提交和精确版本；Web 包使用封存制品的真实 SHA-512 完整性。每次正式发行从这些精确输入独立构建并验证最终制品，源码检查和 Linux 发行物验收分别记录。

Xczs 使用 `serve_path`、`data_dir`、`auth` 等 snake_case JSON 字段；命令行共享根为 `--serve-path /absolute/shared-root`。状态目录与共享根均需完整备份；人工标签、管理员文件和上传/操作状态不可由文件扫描恢复。

`init` 同时创建 `data_dir/logs` 私有目录；`run` 验证该目录后写入共享 JSON 日志。默认单文件上限 8 MiB、保留 4 个归档，总上限 40 MiB。配置和状态命令不打开运行日志文件。
