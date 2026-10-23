// 发布版不弹控制台窗口：Windows 桌面应用不该有黑框。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    prompt_pad_lib::run()
}
