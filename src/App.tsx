import { useCallback, useEffect, useRef, useState } from "react";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { getConfig, optimizePrompt, savePeRecord, setPanelView, submitFeedback, togglePanel } from "./api/commands";
import { AppError } from "./api/client";
import { usePanelState } from "./hooks/usePanelState";
import { usePadStore } from "./state/store";
import { Dot } from "./components/Dot";
import { Panel, PromptInput } from "./components/Panel";
import { ResultView } from "./components/ResultView";
import { Settings } from "./components/Settings";
import { StatusBar } from "./components/StatusBar";
import type { SavePeRecordResponse } from "./api/types";

const DEFAULT_DOMAIN = "general";

/** 把任意异常收敛成可展示的 (message, code)。 */
function toErrorPair(err: unknown): [string, string] {
  if (err instanceof AppError) {
    return [err.message, err.code];
  }
  return [err instanceof Error ? err.message : "未知错误", "internal_error"];
}

export default function App() {
  const mode = usePanelState();
  const { draft, result, status, error, errorCode, config, view } = usePadStore();
  const setDraft = usePadStore((s) => s.setDraft);
  const setResult = usePadStore((s) => s.setResult);
  const setStatus = usePadStore((s) => s.setStatus);
  const setError = usePadStore((s) => s.setError);
  const setConfig = usePadStore((s) => s.setConfig);
  const setView = usePadStore((s) => s.setView);

  const inputRef = useRef<HTMLTextAreaElement>(null);
  const lastRecordRef = useRef<SavePeRecordResponse | null>(null);
  const [copied, setCopied] = useState(false);
  const [savingRecord, setSavingRecord] = useState(false);
  const [rating, setRating] = useState<number | null>(null);
  const clipboardTried = useRef(false);

  useEffect(() => {
    void getConfig()
      .then(setConfig)
      .catch((err: unknown) => {
        const [message, code] = toErrorPair(err);
        setError(message, code);
      });
  }, [setConfig, setError]);

  useEffect(() => {
    if (mode === "expanded" && view === "prompt") {
      inputRef.current?.focus();
      if (!clipboardTried.current && draft.trim().length === 0) {
        clipboardTried.current = true;
        void readText()
          .then((text) => {
            const next = text.trim();
            if (next.length > 0 && usePadStore.getState().draft.trim().length === 0) {
              setDraft(next);
            }
          })
          .catch(() => undefined);
      }
    }
    if (mode === "collapsed") {
      clipboardTried.current = false;
    }
  }, [mode, view, draft, setDraft]);

  useEffect(() => {
    void setPanelView(view).catch(() => undefined);
  }, [view]);

  const handleExpand = useCallback(() => {
    void togglePanel("expand").catch((err: unknown) => {
      const [message, code] = toErrorPair(err);
      setError(message, code);
    });
  }, [setError]);

  const handleCollapse = useCallback(() => {
    void togglePanel("collapse").catch((err: unknown) => {
      const [message, code] = toErrorPair(err);
      setError(message, code);
    });
  }, [setError]);

  const handleOptimize = useCallback(() => {
    const prompt = draft.trim();
    if (prompt.length === 0) {
      return;
    }
    if (!config || config.needs_setup) {
      setView("settings");
      return;
    }

    lastRecordRef.current = null;
    setRating(null);
    setSavingRecord(true);
    setStatus("running");
    setError(null);

    void optimizePrompt({ original_prompt: prompt, domain: DEFAULT_DOMAIN, memory_ids: null })
      .then(async (response) => {
        setResult(response);
        setStatus("success");

        try {
          const record = await savePeRecord({
            original_prompt: prompt,
            optimized_prompt: response.optimized_prompt,
            domain: response.domain || DEFAULT_DOMAIN,
            signals: [`S-${(response.domain || DEFAULT_DOMAIN).replace(/-/g, "").toUpperCase()}`],
            rating: null,
            injected_memory_ids: response.injected_memory_ids,
          });
          lastRecordRef.current = record;
        } catch (err: unknown) {
          const [message, code] = toErrorPair(err);
          setError(`改写成功但落库失败：${message}`, code);
        } finally {
          setSavingRecord(false);
        }
      })
      .catch((err: unknown) => {
        const [message, code] = toErrorPair(err);
        setStatus("error");
        setResult(null);
        setSavingRecord(false);
        setError(message, code);
      });
  }, [draft, config, setError, setResult, setStatus, setView]);

  const handleCopy = useCallback(
    (andCollapse: boolean) => {
      const text = result?.optimized_prompt ?? "";
      if (text.length === 0) {
        return;
      }
      void writeText(text)
        .then(() => {
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1500);
          if (andCollapse) {
            handleCollapse();
          }
        })
        .catch((err: unknown) => {
          const [message, code] = toErrorPair(err);
          setError(`复制失败：${message}`, code);
        });
    },
    [result, setError, handleCollapse],
  );

  const handleRate = useCallback(
    (nextRating: number) => {
      if (rating !== null || savingRecord) {
        return;
      }
      const recordId = lastRecordRef.current?.record_id;
      if (!recordId) {
        setError("这条改写尚未落库，无法评分", "storage_write_failed");
        return;
      }
      setRating(nextRating);
      void submitFeedback({ record_id: recordId, rating: nextRating, note: null }).catch(
        (err: unknown) => {
          setRating(null);
          const [message, code] = toErrorPair(err);
          setError(`评分提交失败：${message}`, code);
        },
      );
    },
    [rating, savingRecord, setError],
  );

  const shortcut = config?.global_shortcut || "Ctrl+Q";

  if (mode === "collapsed") {
    return (
      <div className="flex h-full w-full items-center justify-center">
        <Dot onToggle={handleExpand} />
      </div>
    );
  }

  if (view === "settings") {
    return (
      <Panel title="设置" subtitle="模型接入 · 记忆预算" onCollapse={handleCollapse}>
        <Settings />
      </Panel>
    );
  }

  return (
    <Panel
      title="Prompt Pad"
      subtitle={`${shortcut} 唤起 · Esc 收起`}
      onCollapse={handleCollapse}
      onOpenSettings={() => setView("settings")}
    >
      <PromptInput
        value={draft}
        onChange={setDraft}
        onSubmit={handleOptimize}
        busy={status === "running"}
        needsSetup={!config || config.needs_setup}
        compact={result !== null}
        inputRef={inputRef}
      />
      <ResultView
        result={result}
        onCopy={() => handleCopy(false)}
        onCopyAndCollapse={() => handleCopy(true)}
        onRate={handleRate}
        copied={copied}
        rating={rating}
        ratingDisabled={savingRecord || !lastRecordRef.current}
      />
      <StatusBar
        config={config}
        status={status}
        error={error}
        errorCode={errorCode}
        onOpenSettings={() => setView("settings")}
      />
    </Panel>
  );
}
