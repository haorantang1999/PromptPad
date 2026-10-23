// geometry.rs 落位算法的等价复刻，用来在没有 Rust 工具链时验算单测断言。
//
// 逐条对应 `src-tauri/src/geometry.rs` 里的 `#[test]`，任何一条不成立就说明
// Rust 侧的单测会红。改动 geometry.rs 后必须重跑本脚本。

const SCREEN = { x: 0, y: 0, width: 1920, height: 1080 };
const DOT_SIZE = 48;
const PANEL_W = 420;
const PANEL_H = 340;

const dotAt = (x, y) => ({ x, y, width: DOT_SIZE, height: DOT_SIZE });
const right = (r) => r.x + r.width;
const bottom = (r) => r.y + r.height;
const centerX = (r) => r.x + Math.floor(r.width / 2);
const centerY = (r) => r.y + Math.floor(r.height / 2);

function clampInto(x, y, width, height, bounds) {
  const maxX = right(bounds) - width;
  const maxY = bottom(bounds) - height;
  const clampedX = maxX < bounds.x ? bounds.x : Math.min(Math.max(x, bounds.x), maxX);
  const clampedY = maxY < bounds.y ? bounds.y : Math.min(Math.max(y, bounds.y), maxY);
  return { x: clampedX, y: clampedY };
}

function rawPosition(dot, panelWidth, panelHeight, direction) {
  switch (direction) {
    case 'Down': return { x: centerX(dot) - Math.floor(panelWidth / 2), y: bottom(dot) };
    case 'Up': return { x: centerX(dot) - Math.floor(panelWidth / 2), y: dot.y - panelHeight };
    case 'Right': return { x: right(dot), y: centerY(dot) - Math.floor(panelHeight / 2) };
    case 'Left': return { x: dot.x - panelWidth, y: centerY(dot) - Math.floor(panelHeight / 2) };
    default: throw new Error(`未知方向 ${direction}`);
  }
}

function resolvePanelPlacement(dot, panelWidth, panelHeight, monitor) {
  const preferLeft = centerX(dot) >= centerX(monitor);
  const preferUp = centerY(dot) >= centerY(monitor);
  const direction =
    preferLeft && preferUp ? 'Left'
      : preferLeft && !preferUp ? 'Down'
        : !preferLeft && preferUp ? 'Up'
          : 'Right';
  const raw = rawPosition(dot, panelWidth, panelHeight, direction);
  const clamped = clampInto(raw.x, raw.y, panelWidth, panelHeight, monitor);
  return { x: clamped.x, y: clamped.y, direction };
}

let failures = 0;
function check(name, fn) {
  try {
    fn();
    console.log(`  ✓ ${name}`);
  } catch (err) {
    failures += 1;
    console.log(`  ✗ ${name} —— ${err.message}`);
  }
}
function assert(cond, message) {
  if (!cond) throw new Error(message || '断言不成立');
}
function assertEq(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message || '不相等'}：期望 ${JSON.stringify(expected)}，实际 ${JSON.stringify(actual)}`);
  }
}

console.log('geometry.rs 单测等价验算');

check('锚点与还原互为逆运算', () => {
  for (const [x, y] of [[0, 0], [100, 200], [1872, 1032], [960, 540]]) {
    // anchor_from_position → resolve_restore_position 的往返
    const spanX = Math.max(SCREEN.width - DOT_SIZE, 1);
    const spanY = Math.max(SCREEN.height - DOT_SIZE, 1);
    const xPct = Math.min(Math.max((x - SCREEN.x) / spanX, 0), 1);
    const yPct = Math.min(Math.max((y - SCREEN.y) / spanY, 0), 1);
    const rx = Math.round(SCREEN.x + spanX * xPct);
    const ry = Math.round(SCREEN.y + spanY * yPct);
    assert(Math.abs(rx - x) <= 1, `x 往返误差 ${x} → ${rx}`);
    assert(Math.abs(ry - y) <= 1, `y 往返误差 ${y} → ${ry}`);
  }
});

check('靠右下的_dot_向左上展开', () => {
  const dot = dotAt(1872 - DOT_SIZE, 1032 - DOT_SIZE);
  const p = resolvePanelPlacement(dot, PANEL_W, PANEL_H, SCREEN);
  assertEq(p.direction, 'Left');
  assert(p.x >= SCREEN.x, 'x 越界');
  assert(p.y >= SCREEN.y, 'y 越界');
  assert(p.x + PANEL_W <= right(SCREEN), '右边越界');
  assert(p.y + PANEL_H <= bottom(SCREEN), '下边越界');
});

check('靠左上的_dot_向右下展开', () => {
  const dot = dotAt(0, 0);
  const p = resolvePanelPlacement(dot, PANEL_W, PANEL_H, SCREEN);
  assertEq(p.direction, 'Right');
  assert(p.x + PANEL_W <= right(SCREEN), '右边越界');
});

check('四象限的展开方向唯一', () => {
  const cases = [
    [[0, 0], 'Right'],
    [[1872, 0], 'Down'],
    [[0, 1032], 'Up'],
    [[1872, 1032], 'Left'],
  ];
  for (const [[x, y], expected] of cases) {
    const p = resolvePanelPlacement(dotAt(x, y), PANEL_W, PANEL_H, SCREEN);
    assertEq(p.direction, expected, `(${x}, ${y}) 的展开方向`);
  }
});

check('dot_贴左缘时面板夹到边缘而不是出屏', () => {
  const dot = dotAt(0, 1032 - DOT_SIZE);
  const p = resolvePanelPlacement(dot, PANEL_W, PANEL_H, SCREEN);
  assertEq(p.direction, 'Up');
  assertEq(p.x, SCREEN.x, '面板应贴在屏幕左缘');
  assert(p.y >= SCREEN.y, 'y 越界');
  assert(p.y + PANEL_H <= bottom(SCREEN), '下边越界');
});

check('面板在任何位置都不出屏', () => {
  const positions = [[0, 0], [1919, 0], [0, 1079], [1919, 1079], [936, 516], [1872, 1032], [0, 1032]];
  for (const [x, y] of positions) {
    const p = resolvePanelPlacement(dotAt(x, y), PANEL_W, PANEL_H, SCREEN);
    assert(p.x >= SCREEN.x && p.y >= SCREEN.y, `左上角越界 (${x}, ${y}) → ${JSON.stringify(p)}`);
    assert(p.x + PANEL_W <= right(SCREEN) && p.y + PANEL_H <= bottom(SCREEN),
      `右下角越界 (${x}, ${y}) → ${JSON.stringify(p)}`);
  }
});

check('副屏偏移后仍不出屏', () => {
  const second = { x: 1920, y: 0, width: 1280, height: 1024 };
  const dot = dotAt(1920 + 1280 - DOT_SIZE, 1024 - DOT_SIZE);
  const p = resolvePanelPlacement(dot, PANEL_W, PANEL_H, second);
  assert(p.x >= second.x, 'x 越界');
  assert(p.y >= second.y, 'y 越界');
  assert(p.x + PANEL_W <= right(second), '右边越界');
  assert(p.y + PANEL_H <= bottom(second), '下边越界');
});

check('负坐标副屏也能正确夹紧', () => {
  const second = { x: -1920, y: 0, width: 1920, height: 1080 };
  const dot = dotAt(-1920, 500);
  const p = resolvePanelPlacement(dot, PANEL_W, PANEL_H, second);
  assert(p.x >= second.x, `面板跑到副屏左侧：${JSON.stringify(p)}`);
});

check('面板比屏幕还大时贴左上角', () => {
  const tiny = { x: 0, y: 0, width: 200, height: 200 };
  const dot = dotAt(100, 100);
  const p = resolvePanelPlacement(dot, 400, 400, tiny);
  assertEq(p.x, tiny.x);
  assertEq(p.y, tiny.y);
});

check('夹紧函数不会产生负坐标', () => {
  const c = clampInto(-500, -500, 100, 100, SCREEN);
  assertEq(c.x, 0);
  assertEq(c.y, 0);
});

console.log(failures === 0 ? '\n全部通过。' : `\n${failures} 条不通过。`);
process.exit(failures === 0 ? 0 : 1);
