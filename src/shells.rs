#![allow(unreachable_patterns)]

// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

use clap_complete::aot::Shell as AotShell;
use clap_complete_nushell::Nushell as NushellShell;
use clap_complete::aot::{Generator};
use clap::{builder::PossibleValue, ValueEnum};

/// Represents the supported shells for command-line completion.
///
/// Wraps the `clap_complete::aot::Shell` and `clap_complete_nushell::Nushell` shells,
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Shell {
    /// Bourne Again `SHell` (bash)
    Bash,
    /// Elvish shell
    Elvish,
    /// Friendly Interactive `SHell` (fish)
    Fish,
    /// `PowerShell`
    PowerShell,
    /// Z `SHell` (zsh)
    Zsh,
    /// NuSHell
    Nushell,
}

// Hand-rolled so it can work even when `derive` feature is disabled
impl clap::ValueEnum for Shell {
    /// Returns the possible values for this enum.
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Shell::Bash,
            Shell::Elvish,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Zsh,
            Shell::Nushell,
        ]
    }

    /// Converts the enum variant to a `PossibleValue`.
    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Shell::Bash => PossibleValue::new("bash"),
            Shell::Elvish => PossibleValue::new("elvish"),
            Shell::Fish => PossibleValue::new("fish"),
            Shell::PowerShell => PossibleValue::new("powershell"),
            Shell::Zsh => PossibleValue::new("zsh"),
            Shell::Nushell => PossibleValue::new("nushell"),
        })
    }
}

impl std::fmt::Display for Shell {
    /// Formats the shell as a string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for Shell {
    type Err = String;

    /// Parses a string into a `Shell` enum variant.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("invalid variant: {s}"))
    }
}

impl TryFrom<AotShell> for Shell {
    type Error = String;
    /// Converts an `AotShell` to a `Shell`.
    fn try_from(shell: AotShell) -> Result<Self, Self::Error> {
        match shell {
            AotShell::Bash => Ok(Shell::Bash),
            AotShell::Elvish => Ok(Shell::Elvish),
            AotShell::Fish => Ok(Shell::Fish),
            AotShell::PowerShell => Ok(Shell::PowerShell),
            AotShell::Zsh => Ok(Shell::Zsh),
            _ => Err("Nushell is not supported in AOT mode".to_string()),
        }
    }
}

impl TryFrom<Shell> for AotShell {
    type Error = String;

    /// Attempts to convert a `Shell` to an `AotShell`.
    fn try_from(shell: Shell) -> Result<Self, Self::Error> {
        match shell {
            Shell::Bash => Ok(AotShell::Bash),
            Shell::Elvish => Ok(AotShell::Elvish),
            Shell::Fish => Ok(AotShell::Fish),
            Shell::PowerShell => Ok(AotShell::PowerShell),
            Shell::Zsh => Ok(AotShell::Zsh),
            Shell::Nushell => Err("Nushell is not supported in AOT mode".to_string()),
        }
    }
}

impl TryFrom<Shell> for NushellShell {
    type Error = String;

    /// Attempts to convert a `Shell` to a `NushellShell`.
    fn try_from(shell: Shell) -> Result<Self, Self::Error> {
        if shell == Shell::Nushell {
            Ok(NushellShell)
        } else {
            Err("Only Nushell can be converted to NushellShell".to_string())
        }
    }
}

impl TryFrom<NushellShell> for Shell {
    type Error = String;
    /// Converts a `NushellShell` to a `Shell`.
    fn try_from(shell: NushellShell) -> Result<Self, String> {
        match shell {
            NushellShell => Ok(Shell::Nushell),
            _ => Err("Only NushellShell can be converted to Shell::Nushell".to_string()),
        }
    }
}

impl Shell  {

    /// Converts the `Shell` enum to a shell enum implementing `clap_complete::Generator` (as a Box pointer).
    pub fn try_to_clap_complete(&self) -> Result<Box<dyn Generator>, String> {
        match self {
            Shell::Bash => Ok(Box::new(AotShell::Bash)),
            Shell::Elvish => Ok(Box::new(AotShell::Elvish)),
            Shell::Fish => Ok(Box::new(AotShell::Fish)),
            Shell::PowerShell => Ok(Box::new(AotShell::PowerShell)),
            Shell::Zsh => Ok(Box::new(AotShell::Zsh)),
            Shell::Nushell => Ok(Box::new(NushellShell)),
        }
    }

    /// Tries to find the shell from a path to its executable.
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Option<Shell> {
        Self::parse_shell_from_path(path.as_ref())
    }

    fn parse_shell_from_path(path: &std::path::Path) -> Option<Shell> {
        let name = path.file_stem()?.to_str()?;
        match name {
            "bash" => Some(Shell::Bash),
            "elvish" => Some(Shell::Elvish),
            "fish" => Some(Shell::Fish),
            "powershell" | "pwsh" | "powershell_ise" => Some(Shell::PowerShell),
            "zsh" => Some(Shell::Zsh),
            "nu" | "nushell" => Some(Shell::Nushell),
            _ => None,
        }
    }

    /// Attempts to find the shell from the `SHELL` environment variable.
    pub fn from_env() -> Option<Shell> {
        if let Some(env_shell) = std::env::var_os("SHELL") {
            Self::parse_shell_from_path(std::path::Path::new(&env_shell))
        } else {
            None
        }
    }
}

impl Generator for Shell {
    /// Returns the file name for the completion file.
    fn file_name(&self, name: &str) -> String {
        let shell_self = self.try_to_clap_complete();
        shell_self.map(|s| s.file_name(name))
            .unwrap_or_else(|_| format!("{name}.nu")) // Default to Nushell if conversion fails
    }

    /// Generates the completion file for the given command and writes it to the provided buffer.
    fn generate(&self, cmd: &clap::Command, buf: &mut dyn std::io::Write) {
        let shell_self = self.try_to_clap_complete();
        shell_self
            .map(|s| s.try_generate(cmd, buf).unwrap_or_else(|e| panic!("failed to write completion file: {}", e)))
            .unwrap_or_else(|_| panic!("failed to write completion file"));
    }

    /// Attempts to generate the completion file for the given command and writes it to the provided buffer.
    fn try_generate(&self, cmd: &clap::Command, buf: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        let shell_self = self.try_to_clap_complete();
        match shell_self {
            Ok(s) => s.try_generate(cmd, buf),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}
