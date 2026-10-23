import { useEffect, useState } from "react";
import { beginDotDrag } from "../api/commands";
import beeDot from "../assets/bee-dot.png";

interface DotProps {
  onToggle: () => void;
}

/**
 * 收起态 DOT —— 小蜜蜂缩略图。
 *
 * 整个图标是一个按钮，同时也是 Tauri 拖拽区：
 * - 按住左键拖 → 系统移动窗口；
 * - 按下又抬起、没挪动 → 展开面板。
 *
 * 为什么点击判定放在后端：
 * 元素一旦是 `data-tauri-drag-region`，Tauri 的拖拽脚本会在 mousedown 里
 * 调 `preventDefault()` 并启动 Windows 的系统拖拽循环，鼠标被系统独占，
 * 浏览器从此**不再派发 mouseup / click**。实测把 click 挂在拖拽区上，
 * 单击毫无反应；而拖拽结束后系统补发的 mouseup 也不会到达页面。
 * 于是「算点击还是算拖拽」只能由后端轮询左键状态来判定：
 * 这里只负责把「按下了」这件事告诉后端，展开与否由后端决定。
 *
 * 尺寸分工：窗口容器 64px、可见蜜蜂约 52px 居中（仍落在 40–56px 区间）。
 * 多出来的透明余量留给橙黄辉光与点击波纹外扩，否则外扩会被窗口边界裁掉。
 * 素材已抠白底；显示用透明 PNG，不再套玻璃圆盘。
 */
export function Dot({ onToggle }: DotProps) {
  const [dragging, setDragging] = useState(false);
  // 波纹用递增 key 重放：CSS 动画不会因为同类元素再次出现而自动重跑，
  // key 变了等于换了一个新元素，动画从头播。
  const [rippleKey, setRippleKey] = useState(0);

  // 拖拽区上收不到 mouseup，拖拽态只能靠后端 promise 复位。
  // 全局 mouseup / blur 只是兜底：promise 迟到或失败时，状态不能永远卡在「拖拽中」。
  useEffect(() => {
    if (!dragging) {
      return;
    }
    const release = () => setDragging(false);
    window.addEventListener("mouseup", release);
    window.addEventListener("blur", release);
    return () => {
      window.removeEventListener("mouseup", release);
      window.removeEventListener("blur", release);
    };
  }, [dragging]);

  return (
    <div
      className={[
        "relative flex h-16 w-16 items-center justify-center",
        dragging ? "dot-orb-dragging" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {/* 圆形辉光：径向渐变淡到 alpha=0，不会在透明窗口上留下方形底板。 */}
      <span
        aria-hidden="true"
        className="dot-glow dot-glow-breathe pointer-events-none absolute inset-0 rounded-full"
      />

      {/*
        `deep`：按钮内部的图片也算命中。
        Tauri 的拖拽脚本里，可点击元素自带该属性时才会被当成拖拽区，
        裸值只认元素自身，img 会挡住拖拽。
      */}
      <button
        type="button"
        data-tauri-drag-region="deep"
        aria-label="展开 Prompt Pad"
        title="Prompt Pad（Ctrl+Q 唤起，按住左键可拖动）"
        onMouseDown={(event) => {
          if (event.button !== 0) {
            return;
          }
          // 先亮起拖拽态：按下瞬间还分不清是点击还是拖拽，等后端给结论再复位。
          setDragging(true);
          // 交给后端判定「点击还是拖拽」；判定为点击时后端会自行展开面板。
          void beginDotDrag()
            .then((outcome) => {
              setDragging(false);
              // 只有原地点击才播波纹：拖拽不该有「点到了」的反馈。
              if (outcome === "clicked") {
                setRippleKey((key) => key + 1);
              }
            })
            .catch(() => {
              // 监视失败时退化成本地点击：宁可多展开一次，也不能点了没反应。
              setDragging(false);
              onToggle();
            });
        }}
        className={[
          "dot-bee-float relative z-10 flex h-[52px] w-[52px] items-center justify-center",
          "bg-transparent transition-[scale] duration-200 ease-out",
          "focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--pad-accent-ring)] focus-visible:rounded-full",
          dragging ? "scale-[1.12] cursor-grabbing" : "cursor-grab",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        <img
          src={beeDot}
          alt=""
          draggable={false}
          className="pointer-events-none h-[52px] w-[52px] select-none object-contain drop-shadow-[0_2px_5px_rgb(120_70_0/0.2)]"
        />
      </button>

      {/*
        波纹画在蜜蜂之上：环从中心扩散、越过轮廓后淡出。
        pointer-events-none 是必须的 —— 它盖住中心，不让点击被挡掉。
      */}
      {rippleKey > 0 && (
        <span
          key={rippleKey}
          aria-hidden="true"
          className="dot-ripple pointer-events-none absolute left-1/2 top-1/2 z-20 h-8 w-8 -translate-x-1/2 -translate-y-1/2 rounded-full"
        />
      )}
    </div>
  );
}
