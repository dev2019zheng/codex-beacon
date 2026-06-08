import { useEffect, useMemo, useState } from "react";
import type { PointerEvent } from "react";
import {
  BeaconSnapshot,
  BeaconSnapshotSource,
  BeaconViewMode,
  CodexTaskSnapshot,
  CodexTaskStatus,
  AlertLevel,
  getBeaconSnapshot,
  startBeaconWindowDrag,
  setBeaconWindowMode,
} from "./beaconApi";
import "./App.css";

const ELECTRIC_MASCOT_THEME_ID = "electric-mascot";

const statusTaskLabels: Record<CodexTaskStatus, string> = {
  running: "进行中",
  completed: "已完成",
  waiting_approval: "等待确认",
  waiting_input: "等待输入",
  failed: "失败",
  idle: "空闲",
  unknown: "未知",
};

const statusCompactLabels: Record<CodexTaskStatus, string> = {
  running: "进行中",
  completed: "已完成",
  waiting_approval: "待确认",
  waiting_input: "待输入",
  failed: "失败",
  idle: "空闲",
  unknown: "未知",
};

const statusHeadlines: Record<CodexTaskStatus, string> = {
  running: "正在处理你的编程任务",
  completed: "任务已完成",
  waiting_approval: "等待你确认",
  waiting_input: "需要你补充信息",
  failed: "任务执行失败",
  idle: "当前没有任务",
  unknown: "状态不可用",
};

const statusDetails: Record<CodexTaskStatus, string> = {
  running: "Codex 正在推进当前开发任务",
  completed: "全部任务已完成，可以查看结果",
  waiting_approval: "Codex 需要你确认后才能继续",
  waiting_input: "Codex 需要更多信息来继续处理",
  failed: "任务遇到错误，需要回到 Codex 查看",
  idle: "Codex Beacon 正在待命",
  unknown: "未能读取当前状态源",
};

const sourceLabels: Record<BeaconSnapshotSource, string> = {
  codex_app: "Codex",
  hooks: "Hooks",
  simulation: "Demo",
};

const statusGlyphs: Record<CodexTaskStatus, string> = {
  running: "●",
  completed: "✓",
  waiting_approval: "⚡",
  waiting_input: "?",
  failed: "!",
  idle: "○",
  unknown: "?",
};

const statusPriority: Record<CodexTaskStatus, number> = {
  waiting_approval: 700,
  waiting_input: 650,
  failed: 600,
  completed: 500,
  running: 400,
  idle: 100,
  unknown: 0,
};

const initialSnapshot: BeaconSnapshot = {
  source: "simulation",
  overallStatus: "unknown",
  alertLevel: "silent",
  activeCount: 0,
  waitingCount: 0,
  tasks: [],
  themes: [],
  updatedAt: new Date().toISOString(),
};

function App() {
  const [snapshot, setSnapshot] = useState<BeaconSnapshot>(initialSnapshot);
  const [viewMode, setViewMode] = useState<BeaconViewMode>("card");
  const [selectedTheme, setSelectedTheme] = useState("minimal-card");
  const [error, setError] = useState<string | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);

  async function refreshSnapshot() {
    setIsRefreshing(true);
    try {
      const next = await getBeaconSnapshot();
      setSnapshot(next);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsRefreshing(false);
    }
  }

  async function toggleViewMode() {
    const nextMode = viewMode === "card" ? "capsule" : "card";
    setViewMode(nextMode);
    try {
      await setBeaconWindowMode(nextMode);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function startWindowDrag() {
    try {
      await startBeaconWindowDrag();
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  useEffect(() => {
    refreshSnapshot();
    void setBeaconWindowMode("card");

    const intervalId = window.setInterval(() => {
      refreshSnapshot();
    }, 60_000);

    return () => window.clearInterval(intervalId);
  }, []);

  return (
    <BeaconHUD
      snapshot={snapshot}
      error={error}
      viewMode={viewMode}
      selectedTheme={selectedTheme}
      isRefreshing={isRefreshing}
      onRefresh={refreshSnapshot}
      onToggleViewMode={toggleViewMode}
      onSelectTheme={setSelectedTheme}
      onStartDrag={startWindowDrag}
    />
  );
}

function BeaconHUD({
  snapshot,
  error,
  viewMode,
  selectedTheme,
  isRefreshing,
  onRefresh,
  onToggleViewMode,
  onSelectTheme,
  onStartDrag,
}: {
  snapshot: BeaconSnapshot;
  error: string | null;
  viewMode: BeaconViewMode;
  selectedTheme: string;
  isRefreshing: boolean;
  onRefresh: () => void;
  onToggleViewMode: () => void;
  onSelectTheme: (themeId: string) => void;
  onStartDrag: () => void;
}) {
  return (
    <main className={`beacon-app theme-${selectedTheme}`} data-mode={viewMode}>
      <section
        className="beacon-window"
        data-alert={snapshot.alertLevel}
        data-status={snapshot.overallStatus}
        aria-label="Codex Beacon status"
      >
        <AnimationLayer />
        {viewMode === "card" ? (
          <BeaconCard
            snapshot={snapshot}
            error={error}
            selectedTheme={selectedTheme}
            isRefreshing={isRefreshing}
            onRefresh={onRefresh}
            onToggleViewMode={onToggleViewMode}
            onSelectTheme={onSelectTheme}
            onStartDrag={onStartDrag}
          />
        ) : (
          <BeaconCapsule
            snapshot={snapshot}
            selectedTheme={selectedTheme}
            isRefreshing={isRefreshing}
            onRefresh={onRefresh}
            onToggleViewMode={onToggleViewMode}
            onStartDrag={onStartDrag}
          />
        )}
      </section>
    </main>
  );
}

function BeaconCard({
  snapshot,
  error,
  selectedTheme,
  isRefreshing,
  onRefresh,
  onToggleViewMode,
  onSelectTheme,
  onStartDrag,
}: {
  snapshot: BeaconSnapshot;
  error: string | null;
  selectedTheme: string;
  isRefreshing: boolean;
  onRefresh: () => void;
  onToggleViewMode: () => void;
  onSelectTheme: (themeId: string) => void;
  onStartDrag: () => void;
}) {
  const visibleTasks = useMemo(() => visibleTaskRows(snapshot), [snapshot]);
  const useMascot = isElectricMascotTheme(selectedTheme);

  return (
    <article
      className="beacon-card"
      data-tauri-drag-region
      onPointerDown={(event) => handleDragPointerDown(event, onStartDrag)}
    >
      <header className="beacon-card-header" data-tauri-drag-region>
        <button
          className="beacon-icon-button"
          type="button"
          title="折叠为胶囊"
          aria-label="折叠为胶囊"
          onClick={onToggleViewMode}
        >
          ⌄
        </button>
        <span className="beacon-dot" aria-hidden="true" />
        <div className="beacon-title-stack">
          <span className="beacon-title">Codex Beacon</span>
        </div>
        <span className="beacon-source-pill">{sourceLabels[snapshot.source]}</span>
        <StatusPill status={snapshot.overallStatus} />
        <time className="beacon-time" dateTime={snapshot.updatedAt}>
          {formatRelativeTime(snapshot.updatedAt)}
        </time>
        <button
          className="beacon-icon-button"
          type="button"
          title="刷新状态"
          aria-label="刷新状态"
          aria-busy={isRefreshing}
          data-refreshing={isRefreshing ? "true" : "false"}
          disabled={isRefreshing}
          onClick={onRefresh}
        >
          ↻
        </button>
      </header>

      <section className="beacon-card-body">
        <div className="beacon-radar" aria-hidden="true">
          {useMascot ? (
            <ElectricMascot
              alertLevel={snapshot.alertLevel}
              mode="card"
              status={snapshot.overallStatus}
            />
          ) : (
            <StatusOrb status={snapshot.overallStatus} size="large" />
          )}
        </div>
        <div className="beacon-status-copy">
          <h1>{statusHeadline(snapshot)}</h1>
          <p>{statusDetail(snapshot)}</p>
          <MetricsStrip snapshot={snapshot} />
        </div>
      </section>

      <footer className="beacon-card-footer">
        <TaskList tasks={visibleTasks} />
        <FooterControls
          snapshot={snapshot}
          selectedTheme={selectedTheme}
          onSelectTheme={onSelectTheme}
        />
      </footer>

      {error ? <p className="beacon-error">{error}</p> : null}
    </article>
  );
}

function BeaconCapsule({
  snapshot,
  selectedTheme,
  isRefreshing,
  onRefresh,
  onToggleViewMode,
  onStartDrag,
}: {
  snapshot: BeaconSnapshot;
  selectedTheme: string;
  isRefreshing: boolean;
  onRefresh: () => void;
  onToggleViewMode: () => void;
  onStartDrag: () => void;
}) {
  const useMascot = isElectricMascotTheme(selectedTheme);

  return (
    <article
      className="beacon-capsule"
      data-tauri-drag-region
      onDoubleClick={onToggleViewMode}
      onPointerDown={(event) => handleDragPointerDown(event, onStartDrag)}
    >
      <button
        className="beacon-capsule-orb-button"
        type="button"
        title="展开为卡片"
        aria-label="展开为卡片"
        onClick={onToggleViewMode}
      >
        {useMascot ? (
          <ElectricMascot
            alertLevel={snapshot.alertLevel}
            mode="capsule"
            status={snapshot.overallStatus}
          />
        ) : (
          <StatusOrb status={snapshot.overallStatus} size="capsule" />
        )}
      </button>
      <strong className="beacon-capsule-status">{statusCompactLabels[snapshot.overallStatus]}</strong>
      <span className="beacon-capsule-divider" aria-hidden="true" />
      <span className="beacon-capsule-summary">{capsuleSummary(snapshot)}</span>
      <time className="beacon-time" dateTime={snapshot.updatedAt}>
        {formatRelativeTime(snapshot.updatedAt)}
      </time>
      <button
        className="beacon-icon-button"
        type="button"
        title="刷新状态"
        aria-label="刷新状态"
        aria-busy={isRefreshing}
        data-refreshing={isRefreshing ? "true" : "false"}
        disabled={isRefreshing}
        onClick={onRefresh}
      >
        ↻
      </button>
    </article>
  );
}

function AnimationLayer() {
  return (
    <>
      <span className="beacon-edge-energy" aria-hidden="true" />
      <span className="beacon-alert-halo" aria-hidden="true" />
      <span className="beacon-completion-ring" aria-hidden="true" />
    </>
  );
}

function StatusOrb({ status, size = "default" }: { status: CodexTaskStatus; size?: "default" | "large" | "capsule" }) {
  return (
    <span className={`beacon-orb beacon-orb-${size}`} aria-label={statusTaskLabels[status]} role="img">
      <span className="beacon-orb-core" />
      <span className="beacon-orb-ring" />
      {size === "large" ? (
        <>
          <span className="beacon-orb-orbit orbit-a" />
          <span className="beacon-orb-orbit orbit-b" />
          <span className="beacon-orb-particle particle-a" />
          <span className="beacon-orb-particle particle-b" />
          <span className="beacon-orb-particle particle-c" />
        </>
      ) : null}
    </span>
  );
}

function ElectricMascot({
  status,
  alertLevel,
  mode,
}: {
  status: CodexTaskStatus;
  alertLevel: AlertLevel;
  mode: "card" | "capsule";
}) {
  return (
    <span
      className={`beacon-mascot beacon-mascot-${mode}`}
      data-mascot-alert={alertLevel}
      data-mascot-status={status}
      aria-label={`Electric Mascot ${statusTaskLabels[status]}`}
      role="img"
    >
      <span className="beacon-mascot-stage" aria-hidden="true">
        <span className="beacon-mascot-shadow" />
        <span className="beacon-mascot-aura" />
        <span className="beacon-mascot-tail" />
        <span className="beacon-mascot-ear mascot-ear-left" />
        <span className="beacon-mascot-ear mascot-ear-right" />
        <span className="beacon-mascot-body">
          <span className="beacon-mascot-face">
            <span className="beacon-mascot-eye mascot-eye-left" />
            <span className="beacon-mascot-eye mascot-eye-right" />
            <span className="beacon-mascot-mouth" />
          </span>
          <span className="beacon-mascot-core" />
        </span>
        <span className="beacon-mascot-foot mascot-foot-left" />
        <span className="beacon-mascot-foot mascot-foot-right" />
        <span className="beacon-mascot-discharge discharge-a" />
        <span className="beacon-mascot-discharge discharge-b" />
        <span className="beacon-mascot-discharge discharge-c" />
        <span className="beacon-mascot-spark spark-a" />
        <span className="beacon-mascot-spark spark-b" />
        <span className="beacon-mascot-spark spark-c" />
        <span className="beacon-mascot-smoke smoke-a" />
        <span className="beacon-mascot-smoke smoke-b" />
        <span className="beacon-mascot-scan" />
      </span>
    </span>
  );
}

function StatusPill({ status }: { status: CodexTaskStatus }) {
  return <span className="beacon-status-pill">{statusCompactLabels[status]}</span>;
}

function MetricsStrip({ snapshot }: { snapshot: BeaconSnapshot }) {
  const failedCount = countTasks(snapshot.tasks, "failed");

  return (
    <div className="beacon-metrics" aria-label="Task counters">
      <Metric value={snapshot.activeCount} label="运行中" active={snapshot.overallStatus === "running"} />
      <Metric
        value={snapshot.waitingCount}
        label="等待中"
        active={snapshot.overallStatus === "waiting_approval" || snapshot.overallStatus === "waiting_input"}
      />
      <Metric value={failedCount} label="失败" active={snapshot.overallStatus === "failed"} />
    </div>
  );
}

function Metric({ value, label, active }: { value: number; label: string; active?: boolean }) {
  return (
    <span className="beacon-metric" data-active={active ? "true" : "false"}>
      <strong>{value}</strong>
      <span>{label}</span>
    </span>
  );
}

function TaskList({ tasks }: { tasks: CodexTaskSnapshot[] }) {
  return (
    <div className="beacon-task-list" aria-label="最近任务">
      {tasks.map((task) => (
        <div className="beacon-task-row" data-task-status={task.status} key={task.id}>
          <span className="beacon-task-icon" aria-hidden="true">
            {statusGlyphs[task.status]}
          </span>
          <span className="beacon-task-title">{task.title}</span>
          <span className="beacon-task-status">{statusTaskLabels[task.status]}</span>
          <time className="beacon-task-time" dateTime={task.updatedAt}>
            {formatRelativeTime(task.updatedAt)}
          </time>
          <span className="beacon-task-chevron" aria-hidden="true">
            ›
          </span>
        </div>
      ))}
    </div>
  );
}

function FooterControls({
  snapshot,
  selectedTheme,
  onSelectTheme,
}: {
  snapshot: BeaconSnapshot;
  selectedTheme: string;
  onSelectTheme: (themeId: string) => void;
}) {
  const taskCount = snapshot.tasks.length;

  return (
    <div className="beacon-footer-controls">
      <select
        className="beacon-theme-select"
        aria-label="theme"
        value={selectedTheme}
        onChange={(event) => onSelectTheme(event.currentTarget.value)}
      >
        {snapshot.themes.map((theme) => (
          <option key={theme.id} value={theme.id}>
            {theme.name}
          </option>
        ))}
      </select>
      <div className="beacon-source-readout" aria-label="状态源">
        <span>{sourceLabels[snapshot.source]}</span>
        <strong>{taskCount}</strong>
        <span>任务</span>
      </div>
    </div>
  );
}

function visibleTaskRows(snapshot: BeaconSnapshot) {
  const tasks =
    snapshot.tasks.length > 0
      ? snapshot.tasks
      : [
          {
            id: "idle",
            title: snapshot.overallStatus === "unknown" ? "状态源不可用" : "暂无活跃任务",
            status: snapshot.overallStatus === "unknown" ? ("unknown" as const) : ("idle" as const),
            detail: statusDetails[snapshot.overallStatus],
            updatedAt: snapshot.updatedAt,
          },
        ];

  return [...tasks]
    .sort((left, right) => statusPriority[right.status] - statusPriority[left.status])
    .slice(0, 4);
}

function handleDragPointerDown(event: PointerEvent<HTMLElement>, onStartDrag: () => void) {
  if (event.button !== 0 || isInteractiveDragTarget(event.target)) {
    return;
  }

  onStartDrag();
}

function isInteractiveDragTarget(target: EventTarget | null) {
  return (
    target instanceof HTMLElement &&
    Boolean(target.closest("button, select, option, input, textarea, [data-no-drag]"))
  );
}

function statusHeadline(snapshot: BeaconSnapshot) {
  return statusHeadlines[snapshot.overallStatus];
}

function statusDetail(snapshot: BeaconSnapshot) {
  return snapshot.tasks[0]?.detail || statusDetails[snapshot.overallStatus];
}

function capsuleSummary(snapshot: BeaconSnapshot) {
  switch (snapshot.overallStatus) {
    case "running":
      return `处理中 ${snapshot.activeCount} 个任务`;
    case "completed":
      return "全部任务已完成";
    case "waiting_approval":
      return "等待确认";
    case "waiting_input":
      return "需要你提供信息";
    case "failed":
      return "任务执行失败";
    case "idle":
      return "暂无任务";
    case "unknown":
      return "状态不可用";
  }
}

function countTasks(tasks: CodexTaskSnapshot[], status: CodexTaskStatus) {
  return tasks.filter((task) => task.status === status).length;
}

function isElectricMascotTheme(themeId: string) {
  return themeId === ELECTRIC_MASCOT_THEME_ID;
}

function formatRelativeTime(value?: string) {
  if (!value) {
    return "now";
  }

  const timestamp = new Date(value).getTime();
  if (Number.isNaN(timestamp)) {
    return "now";
  }

  const diffMs = Math.max(0, Date.now() - timestamp);
  const diffMinutes = Math.floor(diffMs / 60_000);
  if (diffMinutes < 1) {
    return "now";
  }
  if (diffMinutes < 60) {
    return `${diffMinutes}m ago`;
  }

  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) {
    return `${diffHours}h ago`;
  }

  return `${Math.floor(diffHours / 24)}d ago`;
}

export default App;
