# Lingjing Client

当前根目录是主工程；`mvp-scaffold/` 保留为原始视觉原型快照。

## 目录

```text
src/
├─ app/                 应用级导航与装配
├─ components/          布局、通用 UI、媒体与任务组件
├─ pages/               页面入口，只负责组合 feature
├─ features/            按聊天/图片/视频/音频/资产拆分的业务模块
├─ services/            网关、任务、资产、持久化适配器
├─ stores/              UI 与领域状态
├─ types/               领域类型和请求模型
└─ lib/                 跨模块工具
```

当前已接入 `MockGatewayAdapter` 和 `MockJobManager`，视频页面可以跑通本地任务状态模拟。后续将用 Tauri Commands、Rust JobManager、SQLite 和系统密钥存储替换这些适配器。

## 开发

```bash
npm install
npm run dev
npm run build
npm run lint
```
