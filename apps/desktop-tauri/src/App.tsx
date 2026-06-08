import { useEffect, useMemo, useState } from "react";
import {
  BeaconSnapshot,
  CodexTaskStatus,
  clearBeaconManualStatus,
  getBeaconSnapshot,
  setBeaconManualStatus,
  statusOptions,
} from "./beaconApi";
import "./App.css";

const statusLabels: Record<CodexTaskStatus, string> = {
  running: "进行中",
  completed: "已完成",
  waiting_approval: "待确认",
  waiting_input: "待输入",
  failed: "失败",
  idle: "空闲",
  unknown: "未知",
};

const statusCopy: Record<CodexTaskStatus, string> = {
  running: "Codex 正在处理任务",
  completed: "任务已完成，可以回来验收",
  waiting_approval: "Codex 正在等待你的确认",
  waiting_input: "Codex 需要你补充输入",
  failed: "任务遇到错误，需要查看",
  idle: "当前没有活跃任务",
  unknown: "暂时没有可用状态源",
};

const statusShortLabels: Record<CodexTaskStatus, string> = {
  running: "▶",
  completed: "✓",
  waiting_approval: "!",
  waiting_input: "?",
  failed: "×",
  idle: "○",
  unknown: "…",
};

const initialSnapshot: BeaconSnapshot = {
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
  const [viewMode, setViewMode] = useState<"card" | "capsule">("card");
  const [selectedTheme, setSelectedTheme] = useState("minimal-card");
  const [error, setError] = useState<string | null>(null);

  async function refreshSnapshot() {
    try {
      const next = await getBeaconSnapshot();
      setSnapshot(next);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function setManualStatus(status: CodexTaskStatus) {
    const next = await setBeaconManualStatus(status);
    setSnapshot(next);
    setError(null);
  }

  async function clearManualStatus() {
    const next = await clearBeaconManualStatus();
    setSnapshot(next);
    setError(null);
  }

  useEffect(() => {
    refreshSnapshot();

    const intervalId = window.setInterval(() => {
      refreshSnapshot();
    }, 60_000);

    return () => window.clearInterval(intervalId);
  }, []);

  const primaryTask = snapshot.tasks[0];
  const updatedTime = useMemo(
    () =>
      new Intl.DateTimeFormat("zh-CN", {
        hour: "2-digit",
        minute: "2-digit",
      }).format(new Date(snapshot.updatedAt)),
    [snapshot.updatedAt],
  );

  return (
    <main className={`beacon-shell ${viewMode}`} data-alert={snapshot.alertLevel}>
      <section className="hud-surface" aria-label="Codex Beacon status">
        <header className="hud-header">
          <button
            className="icon-button"
            type="button"
            title={viewMode === "card" ? "折叠为胶囊" : "展开为卡片"}
            aria-label={viewMode === "card" ? "折叠为胶囊" : "展开为卡片"}
            onClick={() => setViewMode(viewMode === "card" ? "capsule" : "card")}
          >
            {viewMode === "card" ? "−" : "+"}
          </button>
          <div className="status-orb" aria-hidden="true" />
          <div className="title-stack">
            <span className="app-name">Codex Beacon</span>
            <span className="status-line">{statusLabels[snapshot.overallStatus]}</span>
          </div>
          <span className="time-chip">{updatedTime}</span>
        </header>

        {viewMode === "card" ? (
          <>
            <div className="summary-row">
              <div>
                <p className="summary-copy">{statusCopy[snapshot.overallStatus]}</p>
                <p className="task-copy">{primaryTask?.detail ?? "等待下一次 1min 状态检查"}</p>
              </div>
              <div className="counter-grid" aria-label="task counters">
                <span>{snapshot.activeCount}</span>
                <span>运行</span>
                <span>{snapshot.waitingCount}</span>
                <span>等待</span>
              </div>
            </div>

            <div className="task-list">
              {(snapshot.tasks.length > 0 ? snapshot.tasks : [{ id: "idle", title: "No active task", status: "idle" as const, detail: "Codex 当前没有活跃任务", updatedAt: snapshot.updatedAt }]).map((task) => (
                <div className="task-row" key={task.id}>
                  <span className="task-status">{statusLabels[task.status]}</span>
                  <span className="task-title">{task.title}</span>
                </div>
              ))}
            </div>

            <div className="control-row">
              <select
                aria-label="theme"
                value={selectedTheme}
                onChange={(event) => setSelectedTheme(event.currentTarget.value)}
              >
                {snapshot.themes.map((theme) => (
                  <option key={theme.id} value={theme.id}>
                    {theme.name}
                  </option>
                ))}
              </select>
              <button type="button" onClick={clearManualStatus}>
                Auto
              </button>
              {statusOptions.map((status) => (
                <button
                  type="button"
                  key={status}
                  title={statusLabels[status]}
                  aria-label={statusLabels[status]}
                  onClick={() => setManualStatus(status)}
                >
                  {statusShortLabels[status]}
                </button>
              ))}
            </div>
          </>
        ) : (
          <div className="capsule-body">
            <span>{statusCopy[snapshot.overallStatus]}</span>
            <button type="button" onClick={refreshSnapshot}>
              ↻
            </button>
          </div>
        )}

        {error ? <p className="error-line">{error}</p> : null}
      </section>
    </main>
  );
}

export default App;
