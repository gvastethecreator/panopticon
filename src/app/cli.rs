//! Command-line parsing and help/version text for the binary entry point.

use crate::StartupArgs;

pub(crate) fn parse_startup_args() -> Result<StartupArgs, String> {
    parse_startup_args_from(std::env::args())
}

fn parse_startup_args_from(
    args: impl IntoIterator<Item = impl Into<String>>,
) -> Result<StartupArgs, String> {
    let mut workspace = None;
    let mut args = args.into_iter().map(Into::into);
    let _ = args.next();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" | "/?" => {
                return Ok(StartupArgs::PrintAndExit {
                    message: cli_usage(),
                    stderr: false,
                });
            }
            "--version" | "-V" => {
                return Ok(StartupArgs::PrintAndExit {
                    message: cli_version(),
                    stderr: false,
                });
            }
            "--workspace" => {
                let Some(raw_workspace) = args.next() else {
                    return Err(panopticon::i18n::t("cli.missing_workspace_value").to_owned());
                };
                workspace = Some(parse_workspace_name(&raw_workspace)?);
            }
            _ => {
                if let Some(raw_workspace) = arg.strip_prefix("--workspace=") {
                    workspace = Some(parse_workspace_name(raw_workspace)?);
                } else {
                    return Err(panopticon::i18n::t_fmt("cli.unknown_argument", &arg));
                }
            }
        }
    }

    Ok(StartupArgs::Run { workspace })
}

fn parse_workspace_name(raw_workspace: &str) -> Result<String, String> {
    match panopticon::settings::validate_workspace_name_input(raw_workspace) {
        panopticon::settings::WorkspaceNameValidation::Valid(workspace_name) => Ok(workspace_name),
        panopticon::settings::WorkspaceNameValidation::Empty => {
            Err(panopticon::i18n::t("settings.workspace_empty_name").to_owned())
        }
        panopticon::settings::WorkspaceNameValidation::Invalid(reason) => Err(reason),
    }
}

pub(crate) fn cli_usage() -> String {
    format!(
        "{} {}\n\n{}\n  panopticon [--workspace <name>]\n  panopticon [--workspace=<name>]\n  panopticon --help\n  panopticon --version\n\n{}\n  --workspace <name>   {}\n  --help, -h, /?     {}\n  --version, -V      {}",
        panopticon::i18n::t("app.name"),
        env!("CARGO_PKG_VERSION"),
        panopticon::i18n::t("cli.usage_heading"),
        panopticon::i18n::t("cli.options_heading"),
        panopticon::i18n::t("cli.workspace_option_help"),
        panopticon::i18n::t("cli.help_option_help"),
        panopticon::i18n::t("cli.help_option_version"),
    )
}

fn cli_version() -> String {
    format!(
        "{} {}",
        panopticon::i18n::t("app.name"),
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_startup_args_from, StartupArgs};

    #[test]
    fn parse_startup_args_supports_workspace_value_forms() {
        assert_eq!(
            parse_startup_args_from(["panopticon", "--workspace", "work"]),
            Ok(StartupArgs::Run {
                workspace: Some("work".to_owned()),
            })
        );

        assert_eq!(
            parse_startup_args_from(["panopticon", "--workspace=focus"]),
            Ok(StartupArgs::Run {
                workspace: Some("focus".to_owned()),
            })
        );
    }

    #[test]
    fn parse_startup_args_supports_help_and_version_flags() {
        let help = parse_startup_args_from(["panopticon", "--help"]);
        assert!(matches!(
            help,
            Ok(StartupArgs::PrintAndExit { stderr: false, .. })
        ));
        assert!(matches!(
            help,
            Ok(StartupArgs::PrintAndExit { ref message, .. }) if message.contains("Usage:")
        ));

        let version = parse_startup_args_from(["panopticon", "--version"]);
        assert!(matches!(
            version,
            Ok(StartupArgs::PrintAndExit { stderr: false, .. })
        ));
        assert!(matches!(
            version,
            Ok(StartupArgs::PrintAndExit { ref message, .. })
                if message.contains(env!("CARGO_PKG_VERSION"))
        ));
    }

    #[test]
    fn parse_startup_args_rejects_unknown_or_invalid_arguments() {
        let missing_value = parse_startup_args_from(["panopticon", "--workspace"]);
        assert!(matches!(missing_value, Err(ref error) if error.contains("Missing value")));

        let invalid_profile = parse_startup_args_from(["panopticon", "--workspace", "???"]);
        assert!(matches!(
            invalid_profile,
            Err(ref error) if error.contains("invalid")
        ));

        let unknown = parse_startup_args_from(["panopticon", "--wat"]);
        assert!(matches!(unknown, Err(ref error) if error.contains("Unknown argument")));
    }
}
