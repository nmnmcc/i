use std::path::Path;
use zed::settings::LspSettings;
use zed_extension_api::{self as zed, LanguageServerId, Result};

struct IExtension;

impl zed::Extension for IExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;
        let configured = settings.binary.and_then(|binary| binary.path);
        let command = match configured {
            Some(path) => {
                // Extension processes do not start in the project directory.
                // Resolve workspace-relative settings explicitly on the host path.
                if Path::new(&path).is_absolute() {
                    path
                } else {
                    format!("{}/{}", worktree.root_path(), path)
                }
            }
            None => worktree.which("i").ok_or_else(|| {
                "i was not found on the worktree PATH; build it and set lsp.ilang.binary.path"
                    .to_owned()
            })?,
        };
        Ok(zed::Command {
            command,
            args: vec!["lsp".to_owned()],
            env: Default::default(),
        })
    }
}

zed::register_extension!(IExtension);
