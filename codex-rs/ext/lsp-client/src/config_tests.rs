use super::*;
use pretty_assertions::assert_eq;

fn raw(command: &str, exts: &[(&str, &str)]) -> LspServerConfig {
    LspServerConfig {
        command: command.to_string(),
        args: Vec::new(),
        env: HashMap::new(),
        extension_to_language: exts
            .iter()
            .map(|(e, l)| (e.to_string(), l.to_string()))
            .collect(),
        workspace_folder: None,
        initialization_options: None,
        startup_timeout_ms: None,
        max_restarts: 3,
    }
}

#[test]
fn normalizes_extensions_and_defaults_workspace_to_cwd() {
    let cfg = ResolvedLspServerConfig::resolve(
        "rust",
        &raw("rust-analyzer", &[("RS", "rust"), (".TOML", "toml")]),
        Path::new("/work"),
    )
    .expect("valid config resolves");

    assert_eq!(cfg.workspace_folder, PathBuf::from("/work"));
    assert_eq!(
        cfg.extension_to_language.get(".rs"),
        Some(&"rust".to_string())
    );
    assert_eq!(
        cfg.extension_to_language.get(".toml"),
        Some(&"toml".to_string())
    );
}

#[test]
fn relative_workspace_folder_is_joined_to_cwd() {
    let mut r = raw("rust-analyzer", &[(".rs", "rust")]);
    r.workspace_folder = Some("sub/dir".to_string());
    let cfg = ResolvedLspServerConfig::resolve("rust", &r, Path::new("/work")).unwrap();
    assert_eq!(cfg.workspace_folder, PathBuf::from("/work/sub/dir"));
}

#[test]
fn rejects_empty_command_and_empty_extensions() {
    assert!(
        ResolvedLspServerConfig::resolve("x", &raw("", &[(".rs", "rust")]), Path::new("/"))
            .is_err()
    );
    assert!(ResolvedLspServerConfig::resolve("x", &raw("ra", &[]), Path::new("/")).is_err());
}
