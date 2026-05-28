use anyhow::Result;
use std::env;
use std::process::Command;

use crate::chromium::resolve_chromium_binary;
use crate::xdg;

pub fn run_doctor() -> Result<()> {
    let mut failed = false;

    let checks = vec![
        ("cargo", vec!["--version"], true),
        ("rustc", vec!["--version"], true),
        ("pkg-config", vec!["--version"], false),
    ];

    println!("Deskify doctor");
    println!("-------------");

    for (cmd, args, critical) in checks {
        match Command::new(cmd).args(args).output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("[ok] {} {}", cmd, version);
            }
            _ => {
                println!(
                    "[{}] {} not found or failed",
                    if critical { "fail" } else { "warn" },
                    cmd
                );
                if critical {
                    failed = true;
                }
            }
        }
    }

    match Command::new("cargo").args(["tauri", "--version"]).output() {
        Ok(output) if output.status.success() => {
            println!("[ok] {}", String::from_utf8_lossy(&output.stdout).trim());
        }
        _ => {
            println!(
                "[fail] cargo tauri not available (install with: cargo install tauri-cli --version \"^2.0.0\")"
            );
            failed = true;
        }
    }

    if let Ok(output) = Command::new("pkg-config")
        .args(["--modversion", "webkit2gtk-4.1"])
        .output()
    {
        if output.status.success() {
            println!(
                "[ok] webkit2gtk-4.1 {}",
                String::from_utf8_lossy(&output.stdout).trim()
            );
        } else {
            println!("[warn] webkit2gtk-4.1 not found via pkg-config");
        }
    }

    match resolve_chromium_binary(None) {
        Ok(path) => println!("[ok] chromium browser found: {}", path.display()),
        Err(_) => println!("[warn] no chromium-based browser found in PATH"),
    }

    let executable_dir = xdg::executable_dir()?;
    let applications_dir = xdg::applications_dir()?;
    let icons_dir = xdg::icons_dir()?;
    println!("[info] executable dir: {}", executable_dir.display());
    println!("[info] applications dir: {}", applications_dir.display());
    println!("[info] icons dir: {}", icons_dir.display());
    if let Ok(exe) = env::current_exe() {
        println!("[info] deskify path: {}", exe.display());
    }

    if failed {
        Err(anyhow::anyhow!("Doctor found critical issues"))
    } else {
        println!("Doctor completed: no critical issues detected.");
        Ok(())
    }
}
