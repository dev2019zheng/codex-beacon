import { invoke } from "@tauri-apps/api/core";

export type CodexTaskStatus =
  | "running"
  | "completed"
  | "waiting_approval"
  | "waiting_input"
  | "failed"
  | "idle"
  | "unknown";

export type AlertLevel = "silent" | "soft" | "normal" | "strong";

export type BeaconSnapshotSource = "hooks" | "manual" | "simulation";

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

const hasTauriRuntime = "__TAURI_INTERNALS__" in window;
let browserTick = 0;
let browserManualStatus: CodexTaskStatus | null = null;

export async function getBeaconSnapshot() {
  if (hasTauriRuntime) {
    return invoke<BeaconSnapshot>("get_beacon_snapshot");
  }

  const status = browserManualStatus ?? statusOptions[browserTick % 5];
  const source = browserManualStatus ? "manual" : "simulation";
  browserTick += 1;

  return browserSnapshot(status, source);
}

export async function setBeaconManualStatus(status: CodexTaskStatus) {
  if (hasTauriRuntime) {
    return invoke<BeaconSnapshot>("set_manual_status", { status });
  }

  browserManualStatus = status;
  return browserSnapshot(status, "manual");
}

export async function clearBeaconManualStatus() {
  if (hasTauriRuntime) {
    return invoke<BeaconSnapshot>("clear_manual_status");
  }

  browserManualStatus = null;
  browserTick = 0;
  return browserSnapshot("running", "simulation");
}

export async function setBeaconWindowMode(mode: BeaconViewMode) {
  if (!hasTauriRuntime) {
    return;
  }

  const [{ getCurrentWindow }, { LogicalSize }] = await Promise.all([
    import("@tauri-apps/api/window"),
    import("@tauri-apps/api/dpi"),
  ]);
  const size = mode === "card" ? new LogicalSize(360, 176) : new LogicalSize(240, 48);

  await getCurrentWindow().setSize(size);
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
