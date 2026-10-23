//! 窗口几何计算。
//!
//! 这里只放纯函数：给定矩形与尺寸，算出坐标。
//! 所有 Tauri / Win32 调用都留在 `services::window_ctl`，
//! 这样「多显示器 + DPI + 边缘自适应 + 绝不出屏」这段最容易出错的逻辑
//! 可以脱离窗口系统用表驱动用例直接断言。

use serde::{Deserialize, Serialize};

/// 屏幕矩形，单位是物理像素。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    pub fn center_x(&self) -> i32 {
        self.x + self.width / 2
    }

    pub fn center_y(&self) -> i32 {
        self.y + self.height / 2
    }
}

/// DOT 的位置锚点。
///
/// 持久化的是「哪块屏 + 屏内百分比坐标」而不是绝对像素：
/// 换分辨率、插拔扩展屏、拖动任务栏之后绝对像素会让 DOT 飘到屏幕外。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    /// 显示器名称，用于在多屏环境下稳定识别同一块屏。
    pub monitor_name: Option<String>,
    /// 显示器在 available_monitors() 中的序号，名称不可用时作为兜底。
    pub monitor_index: usize,
    /// 屏内百分比横坐标，0.0 = 屏幕左缘，1.0 = 屏幕右缘。
    pub x_pct: f64,
    /// 屏内百分比纵坐标，0.0 = 屏幕上缘，1.0 = 屏幕下缘。
    pub y_pct: f64,
}

impl Default for Anchor {
    fn default() -> Self {
        Self {
            monitor_name: None,
            monitor_index: 0,
            x_pct: 0.9,
            y_pct: 0.5,
        }
    }
}

/// 面板相对 DOT 的展开方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandDirection {
    Down,
    Up,
    Right,
    Left,
}

/// 面板最终落位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub direction: ExpandDirection,
}

/// 把 DOT 的绝对坐标换算成锚点百分比。
///
/// 百分比的基准是「屏幕可用区域减去 DOT 自身尺寸」，
/// 这样 x_pct = 0.0 正好是 DOT 贴屏幕左缘、1.0 正好贴右缘，
/// 还原时才能精确往返。
pub fn anchor_from_position(
    monitor_name: Option<String>,
    monitor_index: usize,
    x: i32,
    y: i32,
    monitor: &Rect,
    dot_width: i32,
    dot_height: i32,
) -> Anchor {
    let span_x = (monitor.width - dot_width).max(1) as f64;
    let span_y = (monitor.height - dot_height).max(1) as f64;

    Anchor {
        monitor_name,
        monitor_index,
        x_pct: ((x - monitor.x) as f64 / span_x).clamp(0.0, 1.0),
        y_pct: ((y - monitor.y) as f64 / span_y).clamp(0.0, 1.0),
    }
}

/// 依据锚点还原 DOT 的绝对坐标。
///
/// 与 `anchor_from_position` 互为逆运算，往返误差不超过 1 像素。
pub fn resolve_restore_position(
    anchor: &Anchor,
    monitor: &Rect,
    dot_width: i32,
    dot_height: i32,
) -> (i32, i32) {
    let span_x = (monitor.width - dot_width).max(1) as f64;
    let span_y = (monitor.height - dot_height).max(1) as f64;

    let x = monitor.x as f64 + span_x * anchor.x_pct.clamp(0.0, 1.0);
    let y = monitor.y as f64 + span_y * anchor.y_pct.clamp(0.0, 1.0);

    (x.round() as i32, y.round() as i32)
}

/// 把窗口左上角夹紧到边界矩形内，保证窗口绝不越界。
///
/// 边界比窗口还小时（例如超宽屏 + 窄面板的极端 DPI 组合），
/// 退化为贴左上角，宁可让面板溢出也不让左上角跑到负坐标。
pub fn clamp_into(x: i32, y: i32, width: i32, height: i32, bounds: &Rect) -> (i32, i32) {
    let max_x = bounds.right() - width;
    let max_y = bounds.bottom() - height;
    let clamped_x = if max_x < bounds.x {
        bounds.x
    } else {
        x.clamp(bounds.x, max_x)
    };
    let clamped_y = if max_y < bounds.y {
        bounds.y
    } else {
        y.clamp(bounds.y, max_y)
    };
    (clamped_x, clamped_y)
}

/// 按指定方向计算面板的原始左上角坐标（未做边界修正）。
fn raw_position(
    dot: &Rect,
    panel_width: i32,
    panel_height: i32,
    direction: ExpandDirection,
) -> (i32, i32) {
    match direction {
        // 面板在 DOT 正下方，水平方向与 DOT 中心对齐。
        ExpandDirection::Down => (dot.center_x() - panel_width / 2, dot.bottom()),
        // 面板在 DOT 正上方。
        ExpandDirection::Up => (dot.center_x() - panel_width / 2, dot.y - panel_height),
        // 面板在 DOT 右侧，垂直方向与 DOT 中心对齐。
        ExpandDirection::Right => (dot.right(), dot.center_y() - panel_height / 2),
        // 面板在 DOT 左侧。
        ExpandDirection::Left => (dot.x - panel_width, dot.center_y() - panel_height / 2),
    }
}

/// 依据 DOT 在屏幕上的相对位置，决定面板朝哪边展开。
///
/// 规则：
/// 1. DOT 在屏幕右半边 → 优先向左展开；在左半边 → 优先向右。
/// 2. DOT 在屏幕下半边 → 优先向上展开；在上半边 → 优先向下。
/// 3. 两个半区条件交叉时落点唯一（面板永远朝屏幕中心一侧展开）：
///    左上 → 右 · 右上 → 下 · 左下 → 上 · 右下 → 左。
/// 4. **面板绝不出屏**优先于方向偏好：DOT 贴在屏幕边缘时面板沿交叉轴会溢出，
///    此时把交叉轴夹到屏内（贴在边缘），而不是让面板的一半跑到屏外。
pub fn resolve_panel_placement(
    dot: &Rect,
    panel_width: i32,
    panel_height: i32,
    monitor: &Rect,
) -> Placement {
    let prefer_left = dot.center_x() >= monitor.center_x();
    let prefer_up = dot.center_y() >= monitor.center_y();

    let direction = match (prefer_left, prefer_up) {
        (true, true) => ExpandDirection::Left,
        (true, false) => ExpandDirection::Down,
        (false, true) => ExpandDirection::Up,
        (false, false) => ExpandDirection::Right,
    };

    let (x, y) = raw_position(dot, panel_width, panel_height, direction);

    // 展开轴（方向决定的轴）在「DOT 位于该半区」的前提下本来就放得下；
    // 会溢出的只有交叉轴（DOT 贴着屏幕边缘），而交叉轴是上下方向共用的，
    // 因此翻到相反方向并不能把交叉轴救回来 —— 直接夹紧即可。
    //
    // 无论如何都要保证面板完整落在屏幕内。
    let (x, y) = clamp_into(x, y, panel_width, panel_height, monitor);
    Placement { x, y, direction }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    const DOT_SIZE: i32 = 48;
    const PANEL_W: i32 = 420;
    const PANEL_H: i32 = 340;

    fn dot_at(x: i32, y: i32) -> Rect {
        Rect::new(x, y, DOT_SIZE, DOT_SIZE)
    }

    #[test]
    fn 锚点与还原互为逆运算() {
        for (x, y) in [(0, 0), (100, 200), (1872, 1032), (960, 540)] {
            let anchor = anchor_from_position(
                Some("DISPLAY1".into()),
                0,
                x,
                y,
                &SCREEN,
                DOT_SIZE,
                DOT_SIZE,
            );
            let (rx, ry) = resolve_restore_position(&anchor, &SCREEN, DOT_SIZE, DOT_SIZE);
            assert!((rx - x).abs() <= 1, "x 往返误差过大: {x} -> {rx}");
            assert!((ry - y).abs() <= 1, "y 往返误差过大: {y} -> {ry}");
        }
    }

    #[test]
    fn 靠右下的_dot_向左上展开() {
        let dot = dot_at(1872 - DOT_SIZE, 1032 - DOT_SIZE);
        let placement = resolve_panel_placement(&dot, PANEL_W, PANEL_H, &SCREEN);
        assert_eq!(placement.direction, ExpandDirection::Left);
        assert!(placement.x >= SCREEN.x);
        assert!(placement.y >= SCREEN.y);
        assert!(placement.x + PANEL_W <= SCREEN.right());
        assert!(placement.y + PANEL_H <= SCREEN.bottom());
    }

    #[test]
    fn 靠左上的_dot_向右下展开() {
        let dot = dot_at(0, 0);
        let placement = resolve_panel_placement(&dot, PANEL_W, PANEL_H, &SCREEN);
        assert_eq!(placement.direction, ExpandDirection::Right);
        assert!(placement.x + PANEL_W <= SCREEN.right());
    }

    #[test]
    fn 四象限的展开方向唯一() {
        // 面板永远朝屏幕中心一侧展开；两个半区条件交叉时落点唯一。
        let cases = [
            ((0, 0), ExpandDirection::Right),
            ((1872, 0), ExpandDirection::Down),
            ((0, 1032), ExpandDirection::Up),
            ((1872, 1032), ExpandDirection::Left),
        ];
        for ((x, y), expected) in cases {
            let placement = resolve_panel_placement(&dot_at(x, y), PANEL_W, PANEL_H, &SCREEN);
            assert_eq!(placement.direction, expected, "({x}, {y}) 的展开方向不对");
        }
    }

    #[test]
    fn dot_贴左缘时面板夹到边缘而不是出屏() {
        // DOT 贴屏幕左缘且靠下：纵向偏下 → 向上展开。面板宽 420 大于 DOT 到左缘的
        // 可用空间，必须把交叉轴夹到左缘，不能让面板有一半跑到屏幕外。
        let dot = dot_at(0, 1032 - DOT_SIZE);
        let placement = resolve_panel_placement(&dot, PANEL_W, PANEL_H, &SCREEN);
        assert_eq!(placement.direction, ExpandDirection::Up);
        assert_eq!(placement.x, SCREEN.x, "面板应贴在屏幕左缘");
        assert!(placement.y >= SCREEN.y);
        assert!(placement.y + PANEL_H <= SCREEN.bottom());
    }

    #[test]
    fn 面板在任何位置都不出屏() {
        let positions = [
            (0, 0),
            (1919, 0),
            (0, 1079),
            (1919, 1079),
            (936, 516),
            (1872, 1032),
            (0, 1032),
        ];
        for (x, y) in positions {
            let dot = dot_at(x, y);
            let placement = resolve_panel_placement(&dot, PANEL_W, PANEL_H, &SCREEN);
            assert!(
                placement.x >= SCREEN.x && placement.y >= SCREEN.y,
                "面板左上角越界: {placement:?}"
            );
            assert!(
                placement.x + PANEL_W <= SCREEN.right() && placement.y + PANEL_H <= SCREEN.bottom(),
                "面板右下角越界: {placement:?}"
            );
        }
    }

    #[test]
    fn 副屏偏移后仍不出屏() {
        // 第二块屏在主屏右边，DOT 落在副屏最右下角。
        let second = Rect::new(1920, 0, 1280, 1024);
        let dot = dot_at(1920 + 1280 - DOT_SIZE, 1024 - DOT_SIZE);
        let placement = resolve_panel_placement(&dot, PANEL_W, PANEL_H, &second);
        assert!(placement.x >= second.x);
        assert!(placement.y >= second.y);
        assert!(placement.x + PANEL_W <= second.right());
        assert!(placement.y + PANEL_H <= second.bottom());
    }

    #[test]
    fn 负坐标副屏也能正确夹紧() {
        // 副屏在主屏左边（x 为负），DOT 贴在副屏左缘。
        let second = Rect::new(-1920, 0, 1920, 1080);
        let dot = dot_at(-1920, 500);
        let placement = resolve_panel_placement(&dot, PANEL_W, PANEL_H, &second);
        assert!(placement.x >= second.x, "面板跑到副屏左侧: {placement:?}");
    }

    #[test]
    fn 面板比屏幕还大时贴左上角() {
        let tiny = Rect::new(0, 0, 200, 200);
        let dot = dot_at(100, 100);
        let placement = resolve_panel_placement(&dot, 400, 400, &tiny);
        assert_eq!(placement.x, tiny.x);
        assert_eq!(placement.y, tiny.y);
    }

    #[test]
    fn 夹紧函数不会产生负坐标() {
        let (x, y) = clamp_into(-500, -500, 100, 100, &SCREEN);
        assert_eq!((x, y), (0, 0));
    }
}
