import type { ReactNode, RefObject } from "react";

interface PanelProps {
  title: string;
  subtitle?: string | undefined;
  onCollapse: () => void;
  /** 打开设置视图。不传就不显示设置按钮。 */
  onOpenSettings?: (() => void) | undefined;
  children: ReactNode;
}

/** 展开态面板外壳。尺寸由 Rust 侧窗口控制决定，这里只负责内容排版。 */
export function Panel({ title, subtitle, onCollapse, onOpenSettings, children }: PanelProps) {
  return (
    <div className="pad-enter flex h-full w-full flex-col overflow-hidden rounded-2xl border border-[var(--pad-border)] bg-[var(--pad-panel-bg)] shadow-[var(--pad-shadow)] backdrop-blur-xl">
      {/*
        标题栏是面板的拖拽区。
        `deep` 让整棵子树（标题 + 副标题）都能拖动；裸值只认元素自身，命中范围太窄。
        关闭按钮显式标 `false`：默认可点击元素会阻断上层拖拽，不标就点不动。
      */}
      <header
        data-tauri-drag-region="deep"
        className="flex shrink-0 items-center justify-between gap-2 border-b border-[var(--pad-border)] px-3.5 py-2.5"
      >
        <div className="min-w-0">
          <h1 className="truncate text-[13px] font-semibold tracking-tight text-[var(--pad-text)]">
            {title}
          </h1>
          {subtitle ? (
            <p className="truncate text-[11px] text-[var(--pad-text-tertiary)]">{subtitle}</p>
          ) : null}
        </div>
        <div className="flex shrink-0 items-center gap-0.5">
          {onOpenSettings ? (
            <button
              type="button"
              data-tauri-drag-region="false"
              onClick={onOpenSettings}
              aria-label="打开设置"
              title="设置"
              className="flex h-7 w-7 items-center justify-center rounded-full text-[var(--pad-text-secondary)] transition-colors hover:bg-[var(--pad-surface)] hover:text-[var(--pad-text)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)]"
            >
              <svg viewBox="0 0 16 16" className="h-3.5 w-3.5" aria-hidden="true">
                <circle cx="8" cy="8" r="2.2" stroke="currentColor" strokeWidth="1.4" fill="none" />
                <path
                  d="M8 1.4v1.8M8 12.8v1.8M14.6 8h-1.8M3.2 8H1.4M12.7 3.3l-1.3 1.3M4.6 11.4l-1.3 1.3M12.7 12.7l-1.3-1.3M4.6 4.6L3.3 3.3"
                  stroke="currentColor"
                  strokeWidth="1.4"
                  strokeLinecap="round"
                />
              </svg>
            </button>
          ) : null}
          <button
            type="button"
            data-tauri-drag-region="false"
            onClick={onCollapse}
            aria-label="收起面板"
            title="收起（Esc）"
            className="flex h-7 w-7 items-center justify-center rounded-full text-[var(--pad-text-secondary)] transition-colors hover:bg-[var(--pad-surface)] hover:text-[var(--pad-text)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)]"
          >
            <svg viewBox="0 0 16 16" className="h-3.5 w-3.5" aria-hidden="true">
              <path
                d="M4 4l8 8M12 4l-8 8"
                stroke="currentColor"
                strokeWidth="1.6"
                strokeLinecap="round"
              />
            </svg>
          </button>
        </div>
      </header>
      <div className="flex min-h-0 flex-1 flex-col">{children}</div>
    </div>
  );
}

interface PromptInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  busy: boolean;
  needsSetup?: boolean | undefined;
  /** 有改写结果时压缩输入区，把高度让给结果预览。 */
  compact?: boolean | undefined;
  inputRef: RefObject<HTMLTextAreaElement>;
}

/** 最小可用输入框：原始提示词 + 一键改写。 */
export function PromptInput({
  value,
  onChange,
  onSubmit,
  busy,
  needsSetup = false,
  compact = false,
  inputRef,
}: PromptInputProps) {
  const canSubmit = value.trim().length > 0 && !busy && !needsSetup;

  return (
    <div
      className={
        compact
          ? "flex min-h-0 max-h-[38%] shrink-0 flex-col gap-2.5 p-3.5"
          : "flex min-h-0 flex-1 flex-col gap-2.5 p-3.5"
      }
    >
      <textarea
        ref={inputRef}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          // Ctrl+Enter 提交：单行提示词场景比点按钮更快。
          if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
            event.preventDefault();
            if (canSubmit) onSubmit();
          }
        }}
        placeholder="写好草稿提示词，Ctrl+Enter 一键改写…"
        spellCheck={false}
        className={[
          "selectable min-h-[72px] resize-none rounded-xl border border-[var(--pad-border)] bg-[var(--pad-surface)] px-3 py-2.5 text-[13px] leading-relaxed text-[var(--pad-text)] placeholder:text-[var(--pad-text-tertiary)] focus:border-[var(--pad-accent)] focus:outline-none focus:shadow-[0_0_0_3px_var(--pad-accent-soft)]",
          compact ? "max-h-[140px]" : "flex-1",
        ].join(" ")}
      />
      <div className="flex shrink-0 items-center gap-2">
        <button
          type="button"
          disabled={!canSubmit}
          onClick={onSubmit}
          className="flex-1 rounded-full bg-[var(--pad-accent)] px-3 py-2 text-[13px] font-semibold tracking-tight text-white transition-colors hover:bg-[var(--pad-accent-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)] disabled:cursor-not-allowed disabled:bg-[var(--pad-surface-strong)] disabled:text-[var(--pad-text-tertiary)]"
        >
          {busy ? "改写中…" : needsSetup ? "请先完成设置" : "一键改写"}
        </button>
      </div>
    </div>
  );
}
