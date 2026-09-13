// Windows'ta release yapısında konsol penceresi açılmasın.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rvmaker_lib::run()
}
