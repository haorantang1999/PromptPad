import type { OptimizeResponse } from "../api/types";

interface ResultViewProps {
  result: OptimizeResponse | null;
  onCopy: () => void;
  onCopyAndCollapse: () => void;
  onRate: (rating: number) => void;
  copied: boolean;
  rating: number | null;
  ratingDisabled: boolean;
}

/** 改写结果区：结果文本 + 复制 + 1-5 评分（评分驱动记忆权重 +1 / 0 / −2）。 */
export function ResultView({
  result,
  onCopy,
  onCopyAndCollapse,
  onRate,
  copied,
  rating,
  ratingDisabled,
}: ResultViewProps) {
  if (!result) {
    return null;
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2 border-t border-[var(--pad-border)] p-3.5">
      <div className="flex shrink-0 items-center justify-between gap-2">
        <span className="text-[11px] font-semibold tracking-wide text-[var(--pad-text-secondary)]">
          改写结果
        </span>
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={onCopy}
            className="rounded-full border border-[var(--pad-border)] px-2.5 py-0.5 text-[11px] font-medium text-[var(--pad-text-secondary)] transition-colors hover:bg-[var(--pad-surface)] hover:text-[var(--pad-text)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)]"
          >
            {copied ? "已复制" : "复制"}
          </button>
          <button
            type="button"
            onClick={onCopyAndCollapse}
            className="rounded-full bg-[var(--pad-accent-soft)] px-2.5 py-0.5 text-[11px] font-semibold text-[var(--pad-accent)] transition-colors hover:bg-[rgb(255_138_0/0.18)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)]"
          >
            复制并收起
          </button>
        </div>
      </div>

      <p className="selectable min-h-0 flex-1 overflow-y-auto whitespace-pre-wrap rounded-xl bg-[var(--pad-accent-soft)] px-3 py-2.5 text-[13px] leading-relaxed text-[var(--pad-text)]">
        {result.optimized_prompt}
      </p>

      <div className="flex shrink-0 items-center justify-between gap-2">
        <span className="truncate text-[10px] text-[var(--pad-text-tertiary)]">
          {result.provider} · {result.model_name} · {result.elapsed_ms}ms
          {result.injected_memory_ids.length > 0
            ? ` · 注入 ${result.injected_memory_ids.length} 条记忆`
            : " · 暂无历史经验"}
        </span>
        <div className="flex shrink-0 items-center gap-1" role="group" aria-label="结果评分">
          {[1, 2, 3, 4, 5].map((score) => {
            const selected = rating === score;
            return (
              <button
                key={score}
                type="button"
                onClick={() => onRate(score)}
                disabled={ratingDisabled || rating !== null}
                aria-label={`评分 ${score} 分`}
                aria-pressed={selected}
                title={score >= 4 ? "有帮助，记忆 +1" : score <= 2 ? "变差，记忆 −2" : "无明显改善"}
                className={[
                  "h-5 w-5 rounded-full text-[11px] font-medium focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)]",
                  selected
                    ? "bg-[var(--pad-accent)] text-white"
                    : "border border-[var(--pad-border)] text-[var(--pad-text-tertiary)] hover:border-[var(--pad-accent)] hover:text-[var(--pad-accent)]",
                  ratingDisabled || rating !== null ? "cursor-not-allowed opacity-50" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                {score}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
