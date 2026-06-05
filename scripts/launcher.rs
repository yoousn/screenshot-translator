#![windows_subsystem = "windows"]

use std::process::Command;
use std::env;

fn main() {
    if let Ok(mut exe_path) = env::current_exe() {
        exe_path.pop(); // remove launcher name
        let target = exe_path.join("release").join("YSN-Screenshot-Translator").join("YsnTrans.exe");
        if target.exists() {
            let args: Vec<String> = env::args().skip(1).collect();
            let target_dir = target.parent().unwrap();
            let _ = Command::new(&target)
                .args(&args)
                .current_dir(target_dir)
                .spawn();
        }
    }
}
