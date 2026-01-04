//! VK Tauri - main entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    vk_tauri_lib::run();
}
