# 部署与日常运维

首次准备程序、目录与数据库见[安装指南](getting-started.md)。本页采用 `/opt/xczs/bin/xczs`、`/etc/xczs/xczs.json`、`/var/lib/xczs` 和 `/srv/xczs`，服务用户为 `xczs`。
配置选项见[CLI 与配置](cli.md)，状态机细节见[运行参考](runtime-reference.md)。

## 1. 安装 systemd 和 HTTPS 网关

源码和完整签名运行包带有 `config/` 与 `deploy/`；单文件下载需另取与程序源码提交一致的样例。
以 root 安装已审阅文件，替换 nginx 示例域名和 TLS 证书位置：

```sh
install -d -o root -g root -m 0755 /etc/nginx/conf.d /etc/nginx/snippets
install -o root -g root -m 0644 deploy/xczs.service /etc/systemd/system/xczs.service
install -o root -g root -m 0644 deploy/nginx-xczs.conf /etc/nginx/conf.d/xczs.conf
install -o root -g root -m 0644 deploy/xczs-proxy.conf /etc/nginx/snippets/xczs-proxy.conf
systemd-analyze verify /etc/systemd/system/xczs.service
nginx -t
```

该网关样例要求 nginx 1.25.1+、HTTP SSL/HTTP2 模块和仍获安全更新的 OpenSSL，HTTP/2 使用 `http2 on;`。
先把站点纳入主机现有 nginx 管理，再加载配置。后端仅允许网关访问；同机不可信进程也需隔离。
路径不同的部署同步调整 JSON、unit 的 `ReadWritePaths` 与主机管理配置。

全新状态已执行 `init` 且配置校验成功后：

```sh
systemctl daemon-reload
systemctl enable --now xczs
systemctl reload-or-restart nginx
ss -ltnp
curl --fail --max-time 10 http://127.0.0.1:5000/readyz
```

预期是 xczs 仅监听配置中的回环地址、readiness 成功，从外部 HTTPS 域名可登录并完成测试文件操作。
unit 中的 `Documentation` 指向 `/usr/share/doc/xczs/operations.md`；若需本机手册，另从对应源码提交安装本文。

## 2. 配置和权限

- 配置：共享根之外的单硬链接普通文件，无扩展 POSIX access ACL。可用 `root:xczs 0640`；`0440/0640` 的组须为服务有效组。
- 状态：`/var/lib/xczs` 属于服务用户，权限 `0700`；私有数据库文件 `0600`。状态目录和共享根互不包含，也不能用挂载别名使数据库暴露到共享根。
- 管理员：`hash-password` 生成 Argon2id PHC，配置的 `auth` 列表声明管理员；普通启动校验已有凭据。
- 文件根：同一共享根仅由一个 xczs 实例管理。应用锁不约束 shell 或其他服务，人工写入放在停服维护窗口。

配置与日志不能和状态数据库及 sidecar 指向同一目录项或文件实体。修改配置后先校验，再在维护窗口重启。

## 3. 状态、日志和停机

```sh
sudo systemctl status xczs --no-pager --full
sudo journalctl -u xczs --since today --no-pager
sudo -u xczs /opt/xczs/bin/xczs status --config /etc/xczs/xczs.json --json
```

`/healthz` 表示存活，`/readyz` 表示最近一次数据库、共享根和空间探针结果。监控重启、5xx/429/507、空间、inode、挂载和登录限流。
运行文件日志位于私有状态下，默认每份 8 MiB、保留四个归档。分享时保留错误码和操作 ID，移除账号、PHC、Cookie、CSRF 与文件内容。

停服使用 `sudo systemctl stop xczs`，确认进程退出后维护文件。应用先停止准入，普通工作和已登记提交共享 30 秒宽限，再给排空 10 秒，日志刷新最多 5 秒；unit 的 120 秒超时为外层边界。强制终止可能留下需要确认的提交结果。

## 4. 当前数据的一致性

状态目录包含 `state.sqlite3`、`tags.db`、管理员文件、日志及必要 sidecar；共享根包含用户文件及受管理的上传、删除和隔离对象。扫描可重建文件索引，人工标签、上传状态和回收任务依赖原状态。
两者应保留一致时间点及共享根绑定。状态库绑定根设备号和 inode，换到不同根的校验可能失败；空状态目录不能代替原实例状态。

服务状态页的在线标签备份写入 `data_dir/tag-backups/`，只覆盖标签数据库，不构成整站文件副本。
当前 `.xczs-upload-stages`、`.xczs-upload-delete-<uuid>.trash` 和 `.xczs-quarantine-<uuid>.hold` 都是内部状态。
隔离对象保留供人工调查；先停服、核对来源和内容，再决定处理方式。

## 5. 故障和安全事件

按[症状表](troubleshooting.md)查看启动、认证、上传和任务结果。结构校验失败时保留数据库及 sidecar，先核对程序与共享根身份。
发现疑似入侵时先限制入口，保全日志、程序摘要与时间线，再按影响范围处理凭据。漏洞使用项目私密报告渠道。

构建、依赖审计、完整签名包与下载验证见[发行指南](releasing.md)。
