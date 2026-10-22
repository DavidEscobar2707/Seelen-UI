use std::fs::create_dir;

fn main() {
    let _ = create_dir("gen");
    tauri_build::build();
}
