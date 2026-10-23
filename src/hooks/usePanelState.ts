import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { togglePanel } from "../api/commands";
import { usePadStore } from "../state/store";
import type { PadView } from "../state/store";
import type { PanelMode } from "../api/types";

/**
 * 面板形态同步。
 *
 * Rust 侧是形态的唯一权威：全局热键、托盘菜单、窗口失焦都会在 Rust 侧改变形态。
 * 前端只监听事件做镜像，绝不自行推导形态，否则热键与 UI 会出现两套真相。
 */
export function usePanelState(): PanelMode {
  const mode = usePadStore((s) => s.mode);
  const setMode = usePadStore((s) => s.setMode);
  const setView = usePadStore((s) => s.setView);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listen<PanelMode>("pad://mode", (event) => {
      if (!disposed) {
        setMode(event.payload);
      }
    })
      .then((fn) => {
        if (disposed) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(() => {
        // 事件通道建立失败属于不可恢复的环境问题：面板仍可用热键操作，只是形态不同步。
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [setMode]);

  // 视图切换：托盘「设置」由后端广播 `pad://view`，前端只做镜像。
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void listen<PadView>("pad://view", (event) => {
      if (disposed) {
        return;
      }
      setView(event.payload);
    })
      .then((fn) => {
        if (disposed) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(() => {
        // 视图事件通道建立失败属于不可恢复的环境问题：托盘「设置」退化为默认提示词视图。
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [setView]);

  // Esc 收起。必须在已展开时才转成 collapse，否则 toggle 会把收起态又展开。
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (usePadStore.getState().mode !== "expanded") return;
      event.preventDefault();
      // 设置页的 Esc 是「返回主面板」，不是收成圆球。
      if (usePadStore.getState().view === "settings") {
        usePadStore.getState().setView("prompt");
        return;
      }
      void togglePanel("collapse").catch(() => undefined);
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return mode;
}
