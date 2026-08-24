# Tauri bridge

`bridge.ts` 是 React 与 `src-tauri/src/commands` 的唯一前端边界。

开发浏览器模式没有 Tauri 注入时，页面继续使用 Mock Gateway 和本地持久化；桌面运行时注入 `window.__TAURI_INVOKE__` 后，再由页面 service 迁移到 Rust commands，不把 `invoke`、文件路径或密钥处理散落到组件中。
