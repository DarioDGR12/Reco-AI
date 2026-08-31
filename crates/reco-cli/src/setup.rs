use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use clap::CommandFactory;
use clap_complete::generate;

use crate::Cli;

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum SetupShell {
    Bash,
    Zsh,
    Fish,
}

impl SetupShell {
    pub fn name(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
        }
    }

    pub fn eval_line(self) -> &'static str {
        match self {
            Self::Bash => r#"eval "$(reco completions bash)""#,
            Self::Zsh => r#"eval "$(reco completions zsh)""#,
            Self::Fish => "reco completions fish | source",
        }
    }

    fn extra_hint(self) -> Option<&'static str> {
        match self {
            Self::Zsh => Some("zsh: fpath=($HOME/.zfunc $fpath) en ~/.zshrc"),
            Self::Bash | Self::Fish => None,
        }
    }
}

impl From<SetupShell> for clap_complete::Shell {
    fn from(value: SetupShell) -> Self {
        match value {
            SetupShell::Bash => Self::Bash,
            SetupShell::Zsh => Self::Zsh,
            SetupShell::Fish => Self::Fish,
        }
    }
}

pub fn write_completions(shell: SetupShell) -> Result<(), String> {
    if !io::stdout().is_terminal() {
        let mut cmd = Cli::command();
        let generator: clap_complete::Shell = shell.into();
        generate(generator, &mut cmd, "reco", &mut io::stdout());
        return Ok(());
    }

    let path = completion_path(shell);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("no pude crear {}: {err}", parent.display()))?;
    }

    match fs::File::create(&path) {
        Ok(mut file) => {
            let mut cmd = Cli::command();
            let generator: clap_complete::Shell = shell.into();
            generate(generator, &mut cmd, "reco", &mut file);
            file.flush()
                .map_err(|err| format!("no pude escribir {}: {err}", path.display()))?;
            println!("completados {} → {}", shell.name(), path.display());
            if let Some(extra) = shell.extra_hint() {
                println!("  {extra}");
            }
            println!("  esta sesión: {}", shell.eval_line());
            Ok(())
        }
        Err(err) => {
            eprintln!("no pude escribir {}: {err}", path.display());
            println!("{}", shell.eval_line());
            Ok(())
        }
    }
}

pub fn completion_path(shell: SetupShell) -> PathBuf {
    match shell {
        SetupShell::Bash => xdg_data_home().join("bash-completion/completions/reco"),
        SetupShell::Zsh => home_dir().join(".zfunc/_reco"),
        SetupShell::Fish => xdg_config_home().join("fish/completions/reco.fish"),
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn xdg_data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".local/share"))
}

fn xdg_config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_paths_are_sensible() {
        let bash = completion_path(SetupShell::Bash);
        assert!(
            bash.ends_with("bash-completion/completions/reco"),
            "{}",
            bash.display()
        );
        let zsh = completion_path(SetupShell::Zsh);
        assert!(zsh.ends_with(".zfunc/_reco"), "{}", zsh.display());
        let fish = completion_path(SetupShell::Fish);
        assert!(
            fish.ends_with("fish/completions/reco.fish"),
            "{}",
            fish.display()
        );
    }

    #[test]
    fn eval_lines_call_existing_completions_command() {
        assert!(SetupShell::Bash
            .eval_line()
            .contains("reco completions bash"));
        assert!(SetupShell::Zsh.eval_line().contains("reco completions zsh"));
        assert!(SetupShell::Fish
            .eval_line()
            .contains("reco completions fish"));
    }
}
