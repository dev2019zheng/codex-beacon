# Codex Beacon 架构方案：Tauri Shell + Rust Core

Date: 2026-06-08
Status: Draft

## 目标

Codex Beacon 是一个 macOS 桌面状态提示工具：它把 Codex 当前任务、确认请求、完成事件和失败状态沉淀为稳定 core，再用可替换 shell/theme 呈现为桌面悬浮窗。

核心设计目标：

- 状态能力独立于 UI，shell 不直接读取 Codex 私有文件。
- 悬浮窗、托盘、设置页和主题都只是 shell，可以替换或新增。
- 初版优先服务 macOS，架构保留跨平台和多 agent 接入能力。
- 状态提醒要低打扰但醒目：进行中常驻，完成/待确认时高亮光晕。

## 推荐技术栈

- Core: Rust crate，负责 Codex 状态采集、状态机、持久化和事件分发。
- Shell: Tauri v2，负责窗口、托盘、设置页、主题加载和 IPC。
- UI theme: Web bundle，推荐 React/Vite 或纯 Web Components。
- macOS 浮窗增强: Tauri window API 起步；如置顶/Spaces/全屏行为不够，再加 macOS 原生 `NSPanel` plugin。
- Storage: SQLite，保存任务快照、事件历史、用户配置和主题设置。
- IPC: Tauri command/event 作为进程内通道；后续可抽成 WebSocket 或 Unix socket，支持独立 daemon。

## 分层边界

```text
codex-beacon
  core/
    beacon-core            # Rust library: status model, collectors, state machine
    beacon-daemon          # Optional later: standalone local service
  apps/
    desktop-tauri          # Tauri shell: window, tray, settings, IPC bridge
  themes/
    minimal-card           # Default quiet card + collapsible capsule
    neon-hud               # Neon completion glow
    electric-mascot        # Built-in original electric mascot theme
  docs/
    architecture-tauri-rust-core.md
    release-pipeline.md
```

### Core

Core 是产品的稳定内核，不依赖具体 UI。它只输出统一状态，不关心主题如何显示。

职责：

- 读取 Codex 状态来源。
- 合并事件流和轮询结果。
- 归一化任务状态。
- 存储最近快照和事件历史。
- 对 shell 提供订阅和查询接口。

非职责：

- 不创建窗口。
- 不处理 CSS、动画或主题资源。
- 不把 UI 文案写死在状态采集逻辑里。

### Shell

Shell 是运行在桌面的宿主应用。Tauri shell 负责把 core 暴露给 UI，并管理系统级能力。

职责：

- 半透明悬浮窗、置顶、拖拽、缩放、位置记忆。
- 托盘菜单、设置页、开机启动配置。
- 主题选择、主题加载和主题生命周期。
- 接收 core 事件并转发给 active theme。
- 在完成/待确认事件上触发系统通知、声音或窗口光晕。

非职责：

- 不直接读取 `~/.codex` 数据文件。
- 不自己推断 Codex 状态。
- 不把某个主题的状态字段扩散到 core。

### Theme

Theme 是可替换皮肤。它只消费 shell 提供的状态快照和事件。

职责：

- 控制视觉表现、布局、动画和音效。
- 声明自身支持的尺寸、窗口形态和设置项。
- 根据事件类型播放不同提醒效果。
- 可以表达不同“提醒人格”，例如安静卡片、霓虹 HUD、原创电气吉祥物。

非职责：

- 不访问文件系统。
- 不调用 Codex CLI。
- 不持久化业务状态。
- 不使用受版权或商标保护的角色素材作为内置默认资源。

## 状态模型

Core 对外只暴露稳定状态枚举，隐藏 Codex 内部实现细节。

```ts
type CodexTaskStatus =
  | "running"
  | "completed"
  | "waiting_approval"
  | "waiting_input"
  | "failed"
  | "idle"
  | "unknown";

type CodexTaskSnapshot = {
  id: string;
  title: string;
  cwd: string | null;
  status: CodexTaskStatus;
  source: "codex_app" | "codex_cli" | "codex_app_server" | "hook" | "unknown";
  startedAt: string | null;
  updatedAt: string;
  completedAt: string | null;
  lastMessage: string | null;
  activeFlags: Array<"waiting_approval" | "waiting_input">;
};

type BeaconSnapshot = {
  generatedAt: string;
  overallStatus: CodexTaskStatus;
  tasks: CodexTaskSnapshot[];
  unreadEvents: BeaconEvent[];
};
```

状态映射建议：

- `turn.status = inProgress` -> `running`
- `thread.status.activeFlags contains waitingOnApproval` -> `waiting_approval`
- `thread.status.activeFlags contains waitingOnUserInput` -> `waiting_input`
- `turn.status = completed` -> `completed`
- `turn.status = failed` -> `failed`
- 无活动 turn 且最近无变化 -> `idle`
- 无法判断 -> `unknown`

## Codex 状态来源

初版采用“事件驱动 + 轮询兜底”。

### 事件驱动

通过 Codex hooks 写入 core 事件入口：

- `UserPromptSubmit`: 新任务/新 turn 开始。
- `PermissionRequest`: 需要确认。
- `Stop`: 当前 turn 结束。
- `SubagentStop`: 子代理结束。

优点：

- 完成/确认事件接近实时。
- 不需要高频扫描 Codex 文件。
- 能补齐 Codex 本地 SQLite 不记录的生命周期细节。

### 轮询兜底

每 60 秒读取本地 Codex 状态：

- `~/.codex/state_*.sqlite`: 线程列表、标题、cwd、更新时间、spawn 关系。
- `~/.codex/sessions/**/*.jsonl`: turn/item 历史和最终消息。
- `~/.codex/process_manager/chat_processes.json`: 长任务命令辅助判断。

优点：

- 不依赖 hooks 一定配置成功。
- 能恢复 app 重启前的任务列表。
- 能展示历史任务和最近完成任务。

### app-server 模式

后续提供可选“托管模式”：用户从 Codex Beacon 发起 Codex 任务，core 通过 `codex app-server` 订阅事件。

适用场景：

- 需要最精准的 `turn/started`、`item/completed`、`turn/completed` 事件。
- 需要 Beacon 作为任务控制中心。

限制：

- 独立 app-server 对已经在 Codex Desktop 中运行的线程通常只能看到 `notLoaded`，不能天然旁路接管现有运行时。

## 主题协议

Theme bundle 建议放在 `themes/<theme-id>/`，由 manifest 描述能力。

```json
{
  "id": "electric-mascot",
  "name": "Electric Mascot",
  "version": "0.1.0",
  "entry": "dist/index.html",
  "defaultWindow": {
    "width": 360,
    "height": 112,
    "transparent": true,
    "alwaysOnTop": true
  },
  "supports": ["compact", "expanded", "completionGlow", "approvalPulse", "mascotAnimation"],
  "alertProfile": {
    "running": "soft",
    "completed": "normal",
    "waiting_approval": "strong",
    "waiting_input": "strong",
    "failed": "normal",
    "idle": "silent"
  }
}
```

Shell 给 theme 注入统一 bridge：

```ts
window.beacon = {
  getSnapshot(): Promise<BeaconSnapshot>;
  subscribe(listener: (event: BeaconEvent) => void): () => void;
  setThemePreference(key: string, value: unknown): Promise<void>;
};
```

Theme 只依赖 `window.beacon`，不依赖 Tauri internals。这样未来可以把同一 theme 放进 Swift/WKWebView shell 或 Web preview。

### 内置主题

MVP 内置三款主题：

- `minimal-card`: 默认主题，小卡片 + 可折叠小胶囊。适合日常工作和看视频时低打扰常驻。
- `neon-hud`: 半透明 HUD，完成时出现霓虹光晕，待确认时出现更明显的脉冲边框。
- `electric-mascot`: 原创电气吉祥物主题。进行中时尾部或徽标轻微发光，完成时开心跳跃，失败时短路冒烟，待确认或等待用户输入时发射强电光提醒。

`electric-mascot` 是原创角色方向，官方仓库不内置或命名任何受版权/商标保护的角色。主题协议允许用户在本地安装自定义主题，但第三方主题加载需要单独的安全提示和权限边界。

### 提醒等级

主题可以声明各状态的提醒等级，shell/core 使用等级控制事件是否持续、是否触发系统通知、是否允许声音。

```text
silent  - 只更新状态，不播放动画
soft    - 轻微呼吸灯或微动效
normal  - 播放一次明显动画，可选系统通知
strong  - 持续强提醒，可周期性重复，适用于阻塞任务的确认/输入状态
```

默认策略：

- `running`: `soft`
- `completed`: `normal`
- `waiting_approval`: `strong`
- `waiting_input`: `strong`
- `failed`: `normal`
- `idle`: `silent`

## macOS 悬浮窗策略

MVP 使用 Tauri window 能力：

- transparent
- decorations false
- always on top
- resizable
- draggable region
- saved position/size

若体验不足，增加 macOS plugin：

- 使用 `NSPanel` 提升浮窗语义。
- 支持跨 Spaces 可见、全屏视频上方可见、非激活状态接收拖拽。
- 使用 vibrancy/blur 时保持主题内容可控。

原则：原生 plugin 只处理窗口行为，不承载业务状态。

## MVP 里程碑

### M0 文档和仓库

- 初始化 Git 仓库和 remote。
- 写入本架构文档。
- 明确 core/shell/theme 边界。

### M1 Core prototype

- Rust 定义状态模型。
- 读取 `~/.codex/state_*.sqlite` 和最近 JSONL。
- 输出 `BeaconSnapshot`。
- 单测覆盖状态映射。

### M2 Tauri shell prototype

- 创建透明置顶悬浮窗。
- 通过 Tauri command 获取 snapshot。
- 每 60 秒刷新。
- 显示任务列表和总体状态。
- 默认 UI 是小卡片，并支持折叠成小胶囊。

### M3 Hooks integration

- 提供 Codex hook 安装脚本。
- hooks 写入 core event queue。
- 完成/待确认事件实时触发 theme animation。

### M4 Theme system

- 实现 theme manifest 加载。
- 内置 `minimal-card`、`neon-hud` 和 `electric-mascot`。
- 设置页支持切换主题。
- `electric-mascot` 使用原创电气吉祥物，不使用第三方 IP 角色素材。

### M5 app-server managed mode

- 可选从 Beacon 发起 Codex thread。
- 订阅 app-server turn/item 事件。
- 将精准事件合并进 core 状态机。

### M6 Release pipeline

- 合入 `master` 触发 GitHub Actions preview workflow。
- preview workflow 移动固定 `preview` tag，覆盖同一个 GitHub prerelease，并上传最新 DMG artifact。
- Git tag `vX.Y.Z` 触发 GitHub Actions formal release workflow。
- macOS runner 构建 Tauri app 并产出 `.dmg` 安装包。
- GitHub Release 自动创建或更新，并上传 DMG artifact；正式版本 release 不覆盖，preview release 可覆盖。
- 支持 Apple Silicon / Intel 两个架构，优先评估 `universal-apple-darwin` 单一 DMG。
- 明确签名/公证 secrets，生产 release 必须走 Developer ID 签名和 notarization。

## 验证策略

Core:

- 状态映射单测。
- JSONL fixture 解析测试。
- SQLite fixture 查询测试。
- hook event 合并测试。

Shell:

- Tauri command 测试。
- 窗口配置 smoke test。
- theme manifest 校验。

Manual QA:

- 同时运行两个 Codex 任务。
- 切换到全屏/视频窗口后确认悬浮窗仍可见。
- 完成任务时触发霓虹光晕。
- 触发 approval/request_user_input 时显示待确认。
- `electric-mascot` 在待确认/等待输入时触发强电光提醒，且不会执行任何 Codex 控制动作。
- 合入 `master` 后，固定 `preview` GitHub prerelease 被刷新，并出现最新 macOS `.dmg` 安装包。
- 推送 `vX.Y.Z` tag 后，GitHub Release 中出现 macOS `.dmg` 安装包。
- 正式 release 的 DMG 经过 macOS code signing 和 notarization；无 Apple 凭据时只允许生成 draft/prerelease 或 CI artifact。

## 决策记录

- 选择 Tauri + Rust Core，而不是 Electron：降低常驻资源占用，core 可作为稳定 Rust library 演进。
- 选择 Web theme，而不是纯 SwiftUI theme：主题替换、动画和社区贡献成本更低。
- 保留 macOS `NSPanel` plugin 作为增强项：Tauri 默认窗口先满足 MVP，原生浮窗行为后置。
- Shell 不直接读 Codex 状态：避免每个主题重复实现不稳定的 Codex 文件解析。
- MVP 内置原创 `electric-mascot` 主题：产品不只提供工具型 HUD，也提供可玩化提醒人格；但官方资源避免使用受保护 IP。
- Release 拆分为 `master` preview 和 `vX.Y.Z` formal：preview 方便每次合入后立刻试用最新 DMG，formal 版本保持不可覆盖、可追溯。

## 待确认问题

- 初版是否只支持 macOS，还是同时保留 Windows/Linux 窗口配置。
- hooks 是否作为默认安装步骤，还是用户显式启用。
- 主题是否允许第三方本地路径加载，若允许需要签名/权限提示。
- 是否需要从 Beacon 直接发起 Codex 任务，还是只做旁路状态 HUD。
- 第一个公开版本是否要求 Apple Developer ID 签名/公证，还是先发布 unsigned preview DMG 供自用验证。
