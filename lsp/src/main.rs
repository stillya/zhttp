mod lsp;

fn main() {
    install_runner();
    lsp::start();
}

fn install_runner() {
    let Some(exe_path) = std::env::current_exe().ok() else {
        return;
    };
    let Some(dir) = exe_path.parent() else {
        return;
    };

    let runner_src = dir.join("zhttp");
    if !runner_src.exists() {
        return;
    }

    let Ok(home) = std::env::var("HOME") else {
        return;
    };

    let dest = std::path::PathBuf::from(home)
        .join(".cargo")
        .join("bin")
        .join("zhttp");
    if dest.exists() {
        return;
    }

    if std::fs::copy(&runner_src, &dest).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
        }
    }
}
