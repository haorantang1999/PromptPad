import type { AppConfig } from "../api/types";
import type { OptimizeStatus } from "../state/store";

interface StatusBarProps {
  config: AppConfig | null;
  status: OptimizeStatus;
  error: string | null;
  errorCode: string | null;
  onOpenSettings: () => void;
}

const STATUS_TEXT: Record<OptimizeStatus, string> = {
  idle: "就绪",
  running: "改写中",
  success: "完成",
  error: "出错",
};

/** 底部状态条：展示当前模型、状态与最近一次错误。 */
export function StatusBar({ config, status, error, errorCode, onOpenSettings }: StatusBarProps) {
  const needsSetup = !config || config.needs_setup || config.config_status !== "ok";
  const setupHint =
    config?.config_status === "malformed"
      ? "配置文件已损坏，请重新填写"
      : needsSetup
        ? "尚未配置模型，点此打开设置"
        : null;
  const showSetup = Boolean(setupHint) && !error;
  const canOpenSettings =
    errorCode === "provider_not_configured" || showSetup;

  return (
    <footer className="flex shrink-0 items-center justify-between gap-2 border-t border-[var(--pad-border)] px-3.5 py-2 text-[10px]">
      <span className="truncate text-[var(--pad-text-tertiary)]">
        {config
          ? `${config.provider} · ${config.model_name || "未配置模型"}`
          : "尚未配置"}
      </span>
      {canOpenSettings ? (
        <button
          type="button"
          onClick={onOpenSettings}
          className="truncate text-[var(--pad-warning)] underline-offset-2 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)]"
          title={error ?? setupHint ?? undefined}
        >
          {error ? `[${errorCode ?? "ERROR"}] ${error}` : setupHint}
        </button>
      ) : (
        <span
          className={
            error
              ? "truncate text-[var(--pad-danger)]"
              : "shrink-0 text-[var(--pad-text-tertiary)]"
          }
          title={error ?? undefined}
        >
          {error ? `[${errorCode ?? "ERROR"}] ${error}` : STATUS_TEXT[status]}
        </span>
      )}
    </footer>
  );
}
