# 初始化 Tauri Rust Core 架构文档

## 背景

用户已在 GitHub 创建 `git@github.com:dev2019zheng/codex-beacon.git`，希望将当前目录初始化为本地仓库，并先把 `Tauri + Rust Core` 的技术方案落到本地文档。

## 目标

- 当前目录成为 `codex-beacon` 的本地 Git 仓库，并绑定 GitHub remote。
- 产出一份可执行的架构方案文档，明确 core、shell、theme 的边界。
- 文档应支持后续把 Codex 状态能力沉淀为 core，并允许悬浮窗 UI 作为可替换皮肤。
- 明确 MVP 是只读状态 HUD，默认小卡片可折叠成小胶囊。
- 将原创电气吉祥物主题纳入内置主题范围，作为待确认/等待输入的强提醒皮肤。
- 明确 `master` 合入覆盖预览版 release，GitHub tag 触发正式 release，并产出 macOS `.dmg` 安装包的流水线要求。

## 非目标

- 本阶段不 scaffold Tauri 项目。
- 本阶段不实现 Rust core、macOS 悬浮窗或主题加载器。
- 本阶段不生成吉祥物视觉资产。
- 本阶段不创建会失败的 GitHub Actions workflow，待 Tauri 项目 scaffold 后再落地 workflow 文件。
- 本阶段不提交或 push，除非用户后续明确要求。

## 验收标准

- `git remote -v` 指向 `git@github.com:dev2019zheng/codex-beacon.git`。
- 当前分支命名为 `master`。
- `docs/architecture-tauri-rust-core.md` 存在，并覆盖架构、状态模型、主题机制、目录建议和里程碑。
- 文档明确 `minimal-card`、`neon-hud`、`electric-mascot` 三个内置主题方向。
- 文档明确 `master` preview release、tag release、DMG artifact、签名/公证 secrets 和 unsigned 验证路径。
