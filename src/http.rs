use zed_extension_api::{self as zed, DownloadedFileType, GithubReleaseOptions, LanguageServerId};

const GITHUB_REPO: &str = "stillya/zhttp";
const LSP_BINARY: &str = "zhttp-lsp";

struct HttpExtension {
    cached_lsp_path: Option<String>,
}

impl zed::Extension for HttpExtension {
    fn new() -> Self {
        let mut ext = Self {
            cached_lsp_path: None,
        };
        if let Ok(path) = ext.download_lsp() {
            ext.cached_lsp_path = Some(path);
        }
        ext
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command, String> {
        let path = match &self.cached_lsp_path {
            Some(p) => p.clone(),
            None => {
                let p = self.download_lsp()?;
                self.cached_lsp_path = Some(p.clone());
                p
            }
        };

        Ok(zed::Command {
            command: path,
            args: vec![],
            env: Default::default(),
        })
    }
}

impl HttpExtension {
    fn download_lsp(&self) -> Result<String, String> {
        let (os, arch) = zed::current_platform();

        let os_str = match os {
            zed::Os::Mac => "darwin",
            zed::Os::Linux => "linux",
            zed::Os::Windows => return Err("Windows is not supported".into()),
        };

        let arch_str = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => return Err("x86 (32-bit) is not supported".into()),
        };

        let release = zed::latest_github_release(
            GITHUB_REPO,
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset_name = format!("{LSP_BINARY}-{arch_str}-{os_str}.tar.gz");
        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no release asset found for {asset_name}"))?;

        let version_dir = format!("{LSP_BINARY}-{}", release.version);
        let binary_path = format!("{version_dir}/{LSP_BINARY}");

        if std::fs::metadata(&binary_path).is_err() {
            zed::download_file(&asset.download_url, &version_dir, DownloadedFileType::GzipTar)?;
            zed::make_file_executable(&binary_path)?;

            let runner_path = format!("{version_dir}/zhttp");
            if std::fs::metadata(&runner_path).is_ok() {
                let _ = zed::make_file_executable(&runner_path);
            }
        }

        Ok(binary_path)
    }
}

zed::register_extension!(HttpExtension);
