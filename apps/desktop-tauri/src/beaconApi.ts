import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

export type CodexTaskStatus =
  | "running"
  | "completed"
  | "waiting_approval"
  | "waiting_input"
  | "failed"
  | "idle"
  | "unknown";

export type AlertLevel = "silent" | "soft" | "normal" | "strong";

export type BeaconSnapshotSource = "codex_app" | "hooks" | "manual" | "simulation";

export type BeaconViewMode = "card" | "capsule";

export type CodexTaskSnapshot = {
  id: string;
  title: string;
  status: CodexTaskStatus;
  detail: string;
  updatedAt: string;
};

export type ThemeDescriptor = {
  id: string;
  name: string;
  description: string;
  supportsMascot: boolean;
};

export type BeaconSnapshot = {
  source: BeaconSnapshotSource;
  overallStatus: CodexTaskStatus;
  alertLevel: AlertLevel;
  activeCount: number;
  waitingCount: number;
  tasks: CodexTaskSnapshot[];
  themes: ThemeDescriptor[];
  updatedAt: string;
};

export const statusOptions: CodexTaskStatus[] = [
  "running",
  "waiting_approval",
  "waiting_input",
  "completed",
  "failed",
  "idle",
  "unknown",
];

let browserTick = 0;
let browserManualStatus: CodexTaskStatus | null = null;

export async function getBeaconSnapshot() {
  const tauriSnapshot = await invokeTauriOrNull<BeaconSnapshot>("get_beacon_snapshot");
  if (tauriSnapshot) {
    return tauriSnapshot;
  }

  const status = browserManualStatus ?? statusOptions[browserTick % 5];
  const source = browserManualStatus ? "manual" : "simulation";
  browserTick += 1;

  return browserSnapshot(status, source);
}

export async function setBeaconManualStatus(status: CodexTaskStatus) {
  const tauriSnapshot = await invokeTauriOrNull<BeaconSnapshot>("set_manual_status", { status });
  if (tauriSnapshot) {
    return tauriSnapshot;
  }

  browserManualStatus = status;
  return browserSnapshot(status, "manual");
}

export async function clearBeaconManualStatus() {
  const tauriSnapshot = await invokeTauriOrNull<BeaconSnapshot>("clear_manual_status");
  if (tauriSnapshot) {
    return tauriSnapshot;
  }

  browserManualStatus = null;
  browserTick = 0;
  return browserSnapshot("running", "simulation");
}

export async function setBeaconWindowMode(mode: BeaconViewMode) {
  try {
    const size = mode === "card" ? new LogicalSize(480, 272) : new LogicalSize(280, 52);

    await getCurrentWindow().setSize(size);
  } catch (cause) {
    if (isMissingTauriRuntimeError(cause)) {
      return;
    }

    throw cause;
  }
}

export async function startBeaconWindowDrag() {
  try {
    await getCurrentWindow().startDragging();
  } catch (cause) {
    if (isMissingTauriRuntimeError(cause)) {
      return;
    }

    throw cause;
  }
}

function browserSnapshot(status: CodexTaskStatus, source: BeaconSnapshotSource): BeaconSnapshot {
  const now = new Date().toISOString();
  const tasks = tasksForStatus(status, now);
  const overallStatus = tasks[0]?.status ?? status;

  return {
    source,
    overallStatus,
    alertLevel: alertLevelForStatus(overallStatus),
    activeCount: tasks.filter((task) => task.status === "running").length,
    waitingCount: tasks.filter((task) => task.status === "waiting_approval" || task.status === "waiting_input").length,
    tasks,
    themes: [
      {
        id: "minimal-card",
        name: "Minimal Card",
        description: "Default compact card and capsule HUD.",
        supportsMascot: false,
      },
      {
        id: "neon-hud",
        name: "Neon HUD",
        description: "Glow-first status shell.",
        supportsMascot: false,
      },
      {
        id: "electric-mascot",
        name: "Electric Mascot",
        description: "Original electric mascot reminder personality.",
        supportsMascot: true,
      },
    ],
    updatedAt: now,
  };
}

function tasksForStatus(status: CodexTaskStatus, updatedAt: string): CodexTaskSnapshot[] {
  switch (status) {
    case "idle":
      return [];
    case "running":
      return [
        task("task-frontend", "悬浮窗外壳", "running", "正在渲染状态视图", updatedAt),
        task("task-core", "Rust 状态核心", "running", "正在归一化 Codex 状态", updatedAt),
        task("task-release", "预览版发布流程", "running", "正在准备 macOS 产物", updatedAt),
        task("task-theme", "主题提醒动效", "running", "正在同步 HUD shell", updatedAt),
      ];
    case "waiting_approval":
      return [
        task("task-release", "预览版发布", "waiting_approval", "等待命令确认", updatedAt),
        task("task-hud", "HUD 打磨", "running", "正在渲染半透明层", updatedAt),
      ];
    case "waiting_input":
      return [task("task-requirements", "主题决策", "waiting_input", "需要输入后继续", updatedAt)];
    case "completed":
      return [task("task-mvp", "MVP 脚手架", "completed", "已准备好验收", updatedAt)];
    case "failed":
      return [task("task-build", "Tauri 构建", "failed", "验证阶段构建失败", updatedAt)];
    case "unknown":
      return [task("task-unknown", "Codex 状态源", "unknown", "暂无可用状态", updatedAt)];
  }
}

function task(
  id: string,
  title: string,
  status: CodexTaskStatus,
  detail: string,
  updatedAt: string,
): CodexTaskSnapshot {
  return {
    id,
    title,
    status,
    detail,
    updatedAt,
  };
}

function alertLevelForStatus(status: CodexTaskStatus): AlertLevel {
  switch (status) {
    case "waiting_approval":
    case "waiting_input":
      return "strong";
    case "completed":
    case "failed":
      return "normal";
    case "running":
      return "soft";
    case "idle":
    case "unknown":
      return "silent";
  }
}

async function invokeTauriOrNull<T>(command: string, args?: Record<string, unknown>): Promise<T | null> {
  try {
    return await invoke<T>(command, args);
  } catch (cause) {
    if (isMissingTauriRuntimeError(cause)) {
      return null;
    }

    throw cause;
  }
}

function isMissingTauriRuntimeError(cause: unknown) {
  const message = cause instanceof Error ? cause.message : String(cause);

  return (
    message.includes("__TAURI_INTERNALS__") ||
    message.includes("__TAURI_IPC__") ||
    message.includes("reading 'invoke'") ||
    message.includes('reading "invoke"')
  );
}
