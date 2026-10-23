import { create } from "zustand";
import type { AppConfigView, OptimizeResponse, PanelMode } from "../api/types";

export type OptimizeStatus = "idle" | "running" | "success" | "error";

/**
 * 面板内视图。
 *
 * 「设置」是同一窗口里的另一个视图而不是新窗口：托盘点「设置」时后端广播
 * `pad://view`，前端据此把内容区换成设置表单（后端仍是唯一权威）。
 */
export type PadView = "prompt" | "settings";

interface PadState {
  /** 面板形态。Rust 侧是唯一权威，这里只是镜像。 */
  mode: PanelMode;
  /** 输入框里的原始提示词草稿。 */
  draft: string;
  /** 最近一次改写结果。 */
  result: OptimizeResponse | null;
  status: OptimizeStatus;
  error: string | null;
  errorCode: string | null;
  config: AppConfigView | null;
  /** 当前展示的视图：提示词或设置。 */
  view: PadView;

  setMode: (mode: PanelMode) => void;
  setDraft: (draft: string) => void;
  setResult: (result: OptimizeResponse | null) => void;
  setStatus: (status: OptimizeStatus) => void;
  setError: (message: string | null, code?: string | null) => void;
  setConfig: (config: AppConfigView) => void;
  setView: (view: PadView) => void;
  resetDraft: () => void;
}

export const usePadStore = create<PadState>((set) => ({
  mode: "collapsed",
  draft: "",
  result: null,
  status: "idle",
  error: null,
  errorCode: null,
  config: null,
  view: "prompt",

  setMode: (mode) => set({ mode }),
  setDraft: (draft) => set({ draft }),
  setResult: (result) => set({ result }),
  setStatus: (status) => set({ status }),
  setError: (error, code = null) => set({ error, errorCode: code }),
  setConfig: (config) => set({ config }),
  setView: (view) => set({ view }),
  resetDraft: () =>
    set({ draft: "", result: null, status: "idle", error: null, errorCode: null }),
}));
