# Xczs 运行包

本目录是可直接部署的 Xczs 运行包。它包含服务端二进制、配置与网关样例、许可证、依赖清单、SBOM、构建记录和校验文件。Web 资源全部嵌入二进制，`WEB-ASSETS.json` 记录资源路径、大小、MIME 和 SHA256，并与其他文件一起受到签名校验保护。完整源码、开发测试及教学手册位于与 `BUILD-ENVIRONMENT.txt` 中 `source_sha` 对应的项目仓库提交。

部署前先验证包内完整性：

```sh
sha256sum --check SHA256SUMS
./xczs --version
./xczs web-assets | cmp - WEB-ASSETS.json
```

`web-assets` 只输出内嵌清单，不启动服务或读取配置。运行包不需要额外的 Web 目录。

复制 `config/xczs.json` 并在共享根之外保存实际配置，权限设为 `0600`。先用 `./xczs hash-password` 生成管理员密码哈希，再按环境修改共享根、状态目录和监听地址。显式执行 `./xczs init --config /absolute/private/xczs.json` 创建管理员状态与数据库；运行使用 `./xczs run --config /absolute/private/xczs.json`，不会初始化空实例。浏览器会话要求 HTTPS；生产边界应使用 `deploy/` 中经过项目检查的 systemd 与 nginx 样例，并按实际主机名、证书和回源地址调整。
