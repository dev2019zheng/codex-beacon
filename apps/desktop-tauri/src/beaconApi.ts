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
        task("task-frontend", "HUD shell", "running", "Building the floating status view", updatedAt),
        task("task-core", "Rust core", "running", "Normalizing simulated Codex state", updatedAt),
      ];
    case "waiting_approval":
      return [
        task("task-release", "Preview release", "waiting_approval", "Waiting for command approval", updatedAt),
        task("task-hud", "HUD polish", "running", "Rendering translucent overlay", updatedAt),
      ];
    case "waiting_input":
      return [task("task-requirements", "Theme decision", "waiting_input", "Needs user input before continuing", updatedAt)];
    case "completed":
      return [task("task-mvp", "MVP scaffold", "completed", "Ready for review", updatedAt)];
    case "failed":
      return [task("task-build", "Tauri build", "failed", "Build failed in verification", updatedAt)];
    case "unknown":
      return [task("task-unknown", "Codex status", "unknown", "No recent state source available", updatedAt)];
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
