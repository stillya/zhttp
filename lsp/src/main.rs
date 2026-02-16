mod lsp;

fn main() {
    install_runner();
    lsp::start();
}

fn install_runner() {
    let Some(exe_path) = std::env::current_exe().ok() else {
        eprintln!("zhttp-lsp: failed to determine current exe path");
        return;
    };
    let Some(dir) = exe_path.parent() else {
        eprintln!("zhttp-lsp: exe path has no parent directory");
        return;
    };

    let runner_src = dir.join("zhttp");
    if !runner_src.exists() {
        eprintln!(
            "zhttp-lsp: runner binary not found at {}",
            runner_src.display()
        );
        return;
    }

    let Ok(home) = std::env::var("HOME") else {
        eprintln!("zhttp-lsp: HOME env var not set");
        return;
    };

    let dest = std::path::PathBuf::from(home)
        .join(".cargo")
        .join("bin")
        .join("zhttp");
    if dest.exists() {
        return;
    }

    match std::fs::copy(&runner_src, &dest) {
        Ok(_) => {
            eprintln!("zhttp-lsp: installed runner to {}", dest.display());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
            }
        }
        Err(e) => {
            eprintln!(
                "zhttp-lsp: failed to copy runner to {}: {}",
                dest.display(),
                e
            );
        }
    }
}
