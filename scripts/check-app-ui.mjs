// 真机 UI 冒烟验收：直接观察运行中的窗口状态并模拟交互。
//
// 为什么需要它：单元测试与静态闸都跑在「进程内」，验证不到
// 「插件配置能不能反序列化」「窗口实际多大」「面板会不会跑出屏幕」
// 「拖拽到底生效没有」这类问题。
// 上一轮就因为 tauri.conf.json 里 plugins.global-shortcut 写成 {}，
// 应用在插件初始化阶段直接 panic —— 55 条单测全绿也照样起不来。
//
// 用法：
//   node scripts/check-app-ui.mjs            # 只体检（不点击）
//   node scripts/check-app-ui.mjs --interact # 额外模拟拖拽 / 热键 / 点击展开
//
// 前置：应用已在运行（pnpm tauri dev 或手工启动 exe）。
//
// 注意：本脚本的测量进程会先切到 per-monitor DPI aware。
// 本机是 125% 缩放，Windows 会把非 DPI-aware 进程看到的坐标虚拟化
// （80 物理像素读成 64），不切换就会把坐标系的差异误判成「窗口漂移」。

import { execFileSync } from "node:child_process";

const INTERACT = process.argv.includes("--interact");

const PS = `
$ErrorActionPreference = 'Stop'
$sig = @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public class Pad {
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);

  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l);
  public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtrW(IntPtr h, int i);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);

  [DllImport("user32.dll")] public static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr clip, MonitorProc cb, IntPtr data);
  public delegate bool MonitorProc(IntPtr mon, IntPtr dc, IntPtr rc, IntPtr data);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct MONINFO {
    public int cbSize;
    public RECT rcMonitor;
    public RECT rcWork;
    public int dwFlags;
  }
  [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr mon, ref MONINFO mi);

  [DllImport("shcore.dll")] public static extern int GetProcessDpiAwareness(IntPtr h, out int awareness);
  [DllImport("kernel32.dll")] public static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);

  public static string WorkArea() {
    string res = "";
    EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, (m, dc, rc, d) => {
      MONINFO mi = new MONINFO();
      mi.cbSize = Marshal.SizeOf(mi);
      GetMonitorInfo(m, ref mi);
      res = string.Format("{0},{1},{2},{3}", mi.rcWork.L, mi.rcWork.T, mi.rcWork.R, mi.rcWork.B);
      return false;
    }, IntPtr.Zero);
    return res;
  }

  public static string Find(uint target) {
    string found = "";
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid != target) return true;
      var t = new StringBuilder(256); GetWindowTextW(h, t, 256);
      if (t.ToString() != "Prompt Pad") return true;
      RECT r; GetWindowRect(h, out r);
      long ex = GetWindowLongPtrW(h, -20).ToInt64();
      found = string.Format("{0}|{1}|{2}|{3}|{4}|{5}|{6}",
        h.ToInt64(), r.L, r.T, r.R, r.B, (r.R-r.L), (r.B-r.T));
      found += "|" + ex.ToString("X") + "|" + (IsWindowVisible(h) ? "1" : "0");
      return false;
    }, IntPtr.Zero);
    return found;
  }

  public static string Awareness(uint target) {
    IntPtr h = OpenProcess(0x1000, false, target);
    if (h == IntPtr.Zero) return "unknown";
    int a;
    int rc = GetProcessDpiAwareness(h, out a);
    CloseHandle(h);
    string[] names = { "dpi_unaware", "system_dpi_aware", "per_monitor_dpi_aware" };
    return (rc == 0 && a >= 0 && a < 3) ? names[a] : ("rc" + rc);
  }

  public static void Click(int x, int y) {
    SetCursorPos(x, y);
    System.Threading.Thread.Sleep(120);
    mouse_event(0x0002, 0, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(60);
    mouse_event(0x0004, 0, 0, 0, IntPtr.Zero);
  }
  public static void Key(byte vk) {
    keybd_event(vk, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(60);
    keybd_event(vk, 0, 2, IntPtr.Zero);
  }
  public static void CtrlKey(byte vk) {
    keybd_event(0x11, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(80);
    keybd_event(vk, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(60);
    keybd_event(vk, 0, 2, IntPtr.Zero);
    System.Threading.Thread.Sleep(80);
    keybd_event(0x11, 0, 2, IntPtr.Zero);
  }
  public static void Drag(int x1, int y1, int x2, int y2) {
    SetCursorPos(x1, y1);
    System.Threading.Thread.Sleep(150);
    mouse_event(0x0002, 0, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(150);
    for (int i = 1; i <= 10; i++) {
      SetCursorPos(x1 + (x2 - x1) * i / 10, y1 + (y2 - y1) * i / 10);
      System.Threading.Thread.Sleep(40);
    }
    System.Threading.Thread.Sleep(200);
    mouse_event(0x0004, 0, 0, 0, IntPtr.Zero);
  }
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }

  // 把窗口挪到候选位置（不改变尺寸、不抢焦点）。
  public static bool Move(IntPtr hwnd, int x, int y) {
    // SWP_NOSIZE(0x0001) | SWP_NOACTIVATE(0x0010) | SWP_NOOWNERZORDER(0x0200)
    return SetWindowPos(hwnd, new IntPtr(-1), x, y, 0, 0, 0x0001 | 0x0010 | 0x0200);
  }
  public static string Hit(int x, int y) {
    IntPtr h = WindowFromPoint(new POINT { X = x, Y = y });
    var sb = new StringBuilder(256); GetClassNameW(h, sb, 256);
    return sb.ToString() + "|" + h.ToInt64();
  }
}
'@
Add-Type -TypeDefinition $sig -Language CSharp
# 测量进程必须自己是 per-monitor DPI aware，否则读到的坐标与被测进程不在同一坐标系。
[Pad]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null

$p = Get-Process prompt-pad -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output 'NOPROC'; exit 2 }
$pid2 = [uint32]$p.Id
$w = [Pad]::Find($pid2)
if (-not $w) { Write-Output 'NOWINDOW'; exit 3 }
Write-Output ("WORK|" + [Pad]::WorkArea())
Write-Output ("AWARE|" + [Pad]::Awareness($pid2))

# —— 归位 ——
# 上一次跑测可能把应用停在展开态；不归位就没法断言「DOT 尺寸」这类收起态指标。
#
# 收起用**全局热键**而不是 Esc：Esc 由 webview 的 keydown 处理，窗口没有键盘焦点时
# 按了也没反应，闸就会在「应用根本没展开」上连环误报（实测踩过）。
# 全局热键不依赖焦点，是这里唯一稳的收起手段。
if ($env:PP_UI_INTERACT -eq '1') {
  $cp = $w -split '\\|'
  if ([int]$cp[5] -gt 100) {
    [Pad]::CtrlKey(0x51)
    for ($i = 0; $i -lt 12; $i++) {
      Start-Sleep -Milliseconds 250
      $w = [Pad]::Find($pid2)
      $cp = $w -split '\\|'
      if ([int]$cp[5] -le 100) { break }
    }
    if ([int]$cp[5] -gt 100) {
      # 热键没生效（极少见）：退回到 Esc，并把窗口点一下让它拿到焦点。
      [Pad]::Key(0x1B)
      Start-Sleep -Milliseconds 1600
      $w = [Pad]::Find($pid2)
    }
  }
}
Write-Output ("PROBE|" + $w)

if ($env:PP_UI_INTERACT -eq '1') {
  $parts = $w -split '\\|'
  $l = [int]$parts[1]; $t = [int]$parts[2]; $r = [int]$parts[3]; $b = [int]$parts[4]
  $hwnd = [IntPtr]([int64]$parts[0])

  # —— 挪到一块没被别的窗口盖住的地方 ——
  # 圆球只有 64×64，正好压在别人的窗口上时鼠标事件根本到不了它（Windows 按 z 序命中）。
  # 测拖拽前先在几个候选点里找一个「命中测试确实落在圆球上」的位置，
  # 否则测出来的失败是环境问题，不是产品缺陷。
  $wa = ([Pad]::WorkArea()) -split ','
  $wl = [int]$wa[0]; $wt = [int]$wa[1]; $wr = [int]$wa[2]; $wb = [int]$wa[3]
  $candidates = @(
    @(($wl + 30), ($wt + 30)),
    @(($wr - 90), ($wt + 30)),
    @(($wl + 30), ($wb - 90)),
    @(($wr - 90), ($wb - 90)),
    @([int](($wl + $wr) / 2), ($wt + 30))
  )
  foreach ($cand in $candidates) {
    [Pad]::Move($hwnd, $cand[0], $cand[1]) | Out-Null
    Start-Sleep -Milliseconds 350
    $cur = ([Pad]::Find($pid2)) -split '\\|'
    $ccx = [int](([int]$cur[1] + [int]$cur[3]) / 2)
    $ccy = [int](([int]$cur[2] + [int]$cur[4]) / 2)
    $hit = [Pad]::Hit($ccx, $ccy)
    if ($hit -like ('*|' + $hwnd.ToInt64())) {
      Write-Output ("PLACED|" + $ccx + "," + $ccy)
      break
    } else {
      Write-Output ("COVERED|" + $ccx + "," + $ccy + "|" + $hit)
    }
  }

  $w = [Pad]::Find($pid2)
  $parts = $w -split '\\|'
  $l = [int]$parts[1]; $t = [int]$parts[2]; $r = [int]$parts[3]; $b = [int]$parts[4]
  $cx = [int](($l + $r) / 2)
  $cy = [int](($t + $b) / 2)

  # —— [1] 拖拽 DOT ——
  # 从圆球正中心拖：中心是指示点，最容易被当成「只能点、不能拖」的地方。
  [Pad]::Drag($cx, $cy, $cx - 180, $cy - 120)
  Start-Sleep -Milliseconds 900
  Write-Output ("PROBE|" + [Pad]::Find($pid2))

  # —— [2] 热键展开 ——
  [Pad]::CtrlKey(0x51)   # Ctrl+Q
  Start-Sleep -Milliseconds 1500
  Write-Output ("PROBE|" + [Pad]::Find($pid2))

  # —— [3] 再按热键收回圆球 ——
  [Pad]::CtrlKey(0x51)
  Start-Sleep -Milliseconds 1500
  Write-Output ("PROBE|" + [Pad]::Find($pid2))

  # —— [4] 展开后拖拽标题栏 ——
  # 用户反馈：面板拖不动，一拖就退回圆球。
  # 因此这里既断言「位置变了」，也断言「形态仍是面板」。
  [Pad]::CtrlKey(0x51)
  Start-Sleep -Milliseconds 1500
  $pe = ([Pad]::Find($pid2)) -split '\\|'
  $el = [int]$pe[1]; $et = [int]$pe[2]
  # 标题栏高约 45 物理像素；标题文字自左侧 12 逻辑像素（15 物理像素）起。
  [Pad]::Drag($el + 60, $et + 22, $el - 120, $et + 122)
  Start-Sleep -Milliseconds 900
  Write-Output ("PROBE|" + [Pad]::Find($pid2))

  # —— [5] 收起后点击 DOT 展开 ——
  [Pad]::CtrlKey(0x51)
  Start-Sleep -Milliseconds 1500
  $pc = ([Pad]::Find($pid2)) -split '\\|'
  $cl = [int]$pc[1]; $ct = [int]$pc[2]; $cr = [int]$pc[3]; $cb = [int]$pc[4]
  [Pad]::Click([int](($cl + $cr) / 2), [int](($ct + $cb) / 2))
  Start-Sleep -Milliseconds 1500
  Write-Output ("PROBE|" + [Pad]::Find($pid2))

  # —— [6] Esc 收起 ——
  [Pad]::Key(0x1B)
  Start-Sleep -Milliseconds 1500
  Write-Output ("PROBE|" + [Pad]::Find($pid2))
}
`;

function runPs(interact) {
  const out = execFileSync(
    "powershell",
    ["-NoProfile", "-Command", `$env:PP_UI_INTERACT = '${interact ? 1 : 0}'; ${PS}`],
    { encoding: "utf8" },
  );
  const lines = out.trim().split(/\r?\n/);
  const res = { probes: [], work: null, aware: null };
  for (const line of lines) {
    const [tag, ...rest] = line.split("|");
    if (tag === "PROBE") {
      const [hwnd, l, t, r, b, w, h, ex, vis] = rest;
      res.probes.push({
        hwnd: Number(hwnd),
        rect: { l: +l, t: +t, r: +r, b: +b },
        size: { w: +w, h: +h },
        exStyle: ex,
        visible: vis === "1",
      });
    } else if (tag === "WORK") {
      res.work = rest.join("|");
    } else if (tag === "AWARE") {
      res.aware = rest.join("|");
    } else if (line === "NOPROC" || line === "NOWINDOW") {
      return { missing: line };
    }
  }
  return res;
}

let failures = 0;
function check(name, ok, detail) {
  if (ok) {
    console.log(`  ✓ ${name}`);
  } else {
    failures += 1;
    console.log(`  ✗ ${name} —— ${detail}`);
  }
}

console.log("真机 UI 冒烟验收");

const state = runPs(INTERACT);
if (state.missing) {
  console.log(`  ✗ 找不到运行中的 prompt-pad（${state.missing}），请先 pnpm tauri dev`);
  process.exit(1);
}

const first = state.probes[0];
const work = state.work ? state.work.split(",").map(Number) : null;
console.log(
  `  窗口 (${first.rect.l},${first.rect.t})-(${first.rect.r},${first.rect.b})  ` +
    `尺寸 ${first.size.w}x${first.size.h}  exStyle 0x${first.exStyle}  ` +
    `被测进程 DPI=${state.aware}  工作区 ${work ? work.join(",") : "?"}`,
);

// 收起态：DOT 尺寸、置顶、不抢焦点。
const exBits = Number.parseInt(first.exStyle, 16);
check("收起态尺寸为 64x64（逻辑）", first.size.w === 64 || first.size.w === 80, `实际 ${first.size.w}x${first.size.h}`);
check("窗口置顶（WS_EX_TOPMOST）", (exBits & 0x00000008) !== 0, `exStyle=0x${first.exStyle}`);
check("收起态不激活窗口（WS_EX_NOACTIVATE）", (exBits & 0x08000000) !== 0, `exStyle=0x${first.exStyle}`);

if (!INTERACT) {
  console.log(failures === 0 ? "\n全部通过。" : `\n${failures} 条不通过。`);
  process.exit(failures === 0 ? 0 : 1);
}

// probes 的顺序与 PowerShell 段一一对应：
//   [0] 归位后   [1] DOT 拖拽后   [2] 热键展开后   [3] 再按热键
//   [4] 拖拽标题栏后   [5] 点击 DOT 展开后   [6] Esc 收起后
// 早期版本整体错位一格，把「拖拽后」当成了初始位，拖拽闸永远自比自、必然失败。
const dragged = state.probes[1];
const afterHotkey = state.probes[2];
const afterHotkey2 = state.probes[3];
const afterPanelDrag = state.probes[4];
const expanded = state.probes[5];
const afterEsc = state.probes[6];

// 拖拽 DOT：必须跟着鼠标走。
// data-tauri-drag-region 走 invoke('plugin:window|start_dragging')，
// 该命令受 ACL 管控 —— 缺 core:window:allow-start-dragging 时会被静默拒绝，
// 用户看到的是「按住圆环拖不动，且没有任何报错」。
check(
  "按住 DOT 可拖动（ACL 授予 start_dragging）",
  dragged && (Math.abs(dragged.rect.l - first.rect.l) > 40 || Math.abs(dragged.rect.t - first.rect.t) > 40),
  dragged ? `原位 (${first.rect.l},${first.rect.t}) → 拖后 (${dragged.rect.l},${dragged.rect.t})` : "窗口消失",
);
// 拖拽不是点击：拖完不能顺手把面板打开。
check(
  "拖拽 DOT 不会误展开面板",
  dragged && dragged.size.w === first.size.w,
  dragged ? `实际 ${dragged.size.w}x${dragged.size.h}` : "窗口消失",
);

check("Ctrl+Q 展开为面板", afterHotkey && afterHotkey.size.w > dragged.size.w, afterHotkey ? `实际 ${afterHotkey.size.w}x${afterHotkey.size.h}` : "窗口消失");
check(
  "再按 Ctrl+Q 收回圆球",
  afterHotkey2 && afterHotkey2.size.w === dragged.size.w,
  afterHotkey2 ? `实际 ${afterHotkey2.size.w}x${afterHotkey2.size.h}` : "窗口消失",
);
check(
  "热键收起后位置不变",
  afterHotkey2 && Math.abs(afterHotkey2.rect.l - dragged.rect.l) <= 2 && Math.abs(afterHotkey2.rect.t - dragged.rect.t) <= 2,
  afterHotkey2 ? `原位 (${dragged.rect.l},${dragged.rect.t}) → 现在 (${afterHotkey2.rect.l},${afterHotkey2.rect.t})` : "窗口消失",
);

// 拖拽面板标题栏：位置要变，形态必须仍是面板。
// 用户反馈「一拖就变回圆球」——那是被 Focused(false) 误判成「点了外部」。
check(
  "展开后按住标题栏可拖动面板",
  afterPanelDrag &&
    (Math.abs(afterPanelDrag.rect.l - afterHotkey.rect.l) > 40 || Math.abs(afterPanelDrag.rect.t - afterHotkey.rect.t) > 40),
  afterPanelDrag ? `原位 (${afterHotkey.rect.l},${afterHotkey.rect.t}) → 拖后 (${afterPanelDrag.rect.l},${afterPanelDrag.rect.t})` : "窗口消失",
);
check(
  "拖拽标题栏后面板不退化成圆球",
  afterPanelDrag && afterPanelDrag.size.w === afterHotkey.size.w,
  afterPanelDrag ? `实际 ${afterPanelDrag.size.w}x${afterPanelDrag.size.h}` : "窗口消失",
);

check("点击 DOT 后展开为面板尺寸", expanded && expanded.size.w > dragged.size.w, expanded ? `实际 ${expanded.size.w}x${expanded.size.h}` : "窗口消失");
if (expanded && work) {
  const [wl, wt, wr, wb] = work;
  check(
    "展开面板完全落在工作区内（绝不出屏）",
    expanded.rect.l >= wl && expanded.rect.t >= wt && expanded.rect.r <= wr && expanded.rect.b <= wb,
    `面板 (${expanded.rect.l},${expanded.rect.t})-(${expanded.rect.r},${expanded.rect.b})  工作区 (${wl},${wt})-(${wr},${wb})`,
  );
  const ex2 = Number.parseInt(expanded.exStyle, 16);
  check("展开态可激活窗口（无 WS_EX_NOACTIVATE）", (ex2 & 0x08000000) === 0, `exStyle=0x${expanded.exStyle}`);
}

check("Esc 收起后回到 DOT 尺寸", afterEsc && afterEsc.size.w === dragged.size.w, afterEsc ? `实际 ${afterEsc.size.w}x${afterEsc.size.h}` : "窗口消失");
// 锚点漂移回归闸：展开再收起后 DOT 必须回到原位。
// 曾因「先 set_position 再 set_size」的顺序问题，Windows 改尺寸时连带移动窗口，
// 落位被顶掉，漂移后的坐标又被 Moved 事件写回锚点，于是 DOT 一轮就飘到屏幕边缘。
check(
  "展开再收起后 DOT 回到原位（锚点不漂移）",
  afterEsc && Math.abs(afterEsc.rect.l - dragged.rect.l) <= 2 && Math.abs(afterEsc.rect.t - dragged.rect.t) <= 2,
  afterEsc ? `原位 (${dragged.rect.l},${dragged.rect.t}) → 现在 (${afterEsc.rect.l},${afterEsc.rect.t})` : "窗口消失",
);

console.log(failures === 0 ? "\n全部通过。" : `\n${failures} 条不通过。`);
process.exit(failures === 0 ? 0 : 1);
