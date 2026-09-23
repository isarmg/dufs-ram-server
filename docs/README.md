# Dufs 文档导航

本目录保存当前实现的功能、协议、开发和运维说明。修改代码时，应同步核对源码、测试和对应文档。

## 当前规范

- [项目工作流程与流程树](project-workflow.md)：启动、认证、路由、文件操作、上传和停机流程。
- [HTTP 与运行时合同](http-runtime-contract.md)：路由、安全预算、Foundation 边界与验证入口。
- [完整功能与取舍清单](feature-inventory-and-tradeoffs.md)：功能边界、依赖关系和删除成本。
- [开发者决策矩阵](feature-decision-matrix.md)：逐项唯一 ID、代码锚点、分类、复杂度、删除后果与验证边界。
- [生产部署、备份、current-only 版本切换与恢复](operations.md)：生产环境的权威操作说明。
- [根目录 README](../README.md)：产品范围、配置和公开协议总览。

## 教学资料

- [从零读懂 Dufs](beginner-guide/README.md)：面向初学者的十章教程和源码阅读路线。

教程为了建立直觉会使用简化例子；涉及安全边界、故障语义或生产参数时，应回到当前规范和测试核对。

安全模型、漏洞报告、备份恢复和事件响应统一归入[运维文档](operations.md)，避免并行文档产生事实漂移。
