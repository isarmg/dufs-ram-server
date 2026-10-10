# xczs 完整运行包

本包包含 `xczs`、`config/`、`deploy/`、本手册、许可证、SBOM、依赖清单、构建记录和校验文件。
Web 资源已内嵌；源码、测试和完整手册通过 `BUILD-ENVIRONMENT.txt` 的 `source_sha` 定位。
运行平台为 Linux AMD64 GNU，需匹配 glibc/动态加载器并提供内核 `openat2`。

## 1. 检查完整性

先按发布渠道提供的信任方式验证外层签名与归档摘要，解压后在包根目录执行：

```sh
sha256sum --check SHA256SUMS
./xczs --version
./xczs web-assets | cmp - WEB-ASSETS.json
```

校验成功，程序身份与预期源码 SHA 一致后继续。`web-assets` 只输出清单，不启动服务。

## 2. 准备配置

创建专用 `xczs` 用户和同名组；以下系统安装命令由 root 执行：

```sh
install -d -o root -g root -m 0755 /opt/xczs/bin
install -d -o root -g xczs -m 0750 /etc/xczs
install -d -o xczs -g xczs -m 0700 /var/lib/xczs
install -d -o xczs -g xczs -m 0750 /srv/xczs
install -o root -g root -m 0755 xczs /opt/xczs/bin/xczs
install -o root -g xczs -m 0640 config/xczs.json.example /etc/xczs/xczs.json
/opt/xczs/bin/xczs hash-password
```

编辑 `/etc/xczs/xczs.json`，填入完整密码哈希并核对共享根、私有状态目录、监听地址。
配置位于共享根之外，为无扩展 access ACL 的单硬链接普通文件。共享根和状态目录分别管理，根由 xczs 独占写入。

## 3. 初始化并运行

```sh
sudo -u xczs /opt/xczs/bin/xczs init --config /etc/xczs/xczs.json
sudo -u xczs /opt/xczs/bin/xczs config validate --config /etc/xczs/xczs.json
sudo -u xczs /opt/xczs/bin/xczs run --config /etc/xczs/xczs.json
```

`init` 只用于全新状态；已有实例执行校验并运行。当前状态库绑定共享根身份，已有实例保留同一组根与状态。

## 4. 通过 HTTPS 使用

按实际域名和证书修改 `deploy/nginx-xczs.conf` 与代理片段，将 HTTPS 请求转发到配置的回环监听地址。
样例要求 nginx 1.25.1+。只允许网关访问后端。浏览器经 HTTPS 登录后上传并下载测试文件，核对实际内容。
需要常驻服务时安装 `deploy/xczs.service`，先检查 `systemd-analyze verify` 和 `nginx -t`，再启用服务。

服务诊断使用 `xczs status --config /etc/xczs/xczs.json --json` 和 systemd Journal。
操作结果未确认时先检查任务与目标文件，保留状态数据库、上传目录和隔离对象。

完整配置、安装和排障步骤见 [项目文档](https://github.com/isarmg/xczs/blob/main/docs/README.md)，应选择与包内源码 SHA 对应的提交阅读。
