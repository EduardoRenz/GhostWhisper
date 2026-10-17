// Impede a abertura de console no Windows em builds de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ghostwhisper_lib::run()
}
