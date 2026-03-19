#![allow(unreachable_patterns)]

// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

use clap::{ValueEnum, builder::PossibleValue};
use clap_complete::aot::Generator;
use clap_complete::aot::Shell as AotShell;
use clap_complete_nushell::Nushell as NushellShell;

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
            Shell::PowerShell => PossibleValue::new("powershell").alias("pwsh"),
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_from_path_bash() {
        assert_eq!(Shell::from_path("/bin/bash"), Some(Shell::Bash));
        assert_eq!(Shell::from_path("bash.exe"), Some(Shell::Bash));
    }

    #[test]
    fn test_from_path_zsh() {
        assert_eq!(Shell::from_path("/usr/bin/zsh"), Some(Shell::Zsh));
        assert_eq!(Shell::from_path("zsh"), Some(Shell::Zsh));
    }

    #[test]
    fn test_from_path_fish() {
        assert_eq!(Shell::from_path("/usr/local/bin/fish"), Some(Shell::Fish));
        assert_eq!(Shell::from_path("fish"), Some(Shell::Fish));
    }

    #[test]
    fn test_from_path_powershell() {
        // Rust's Path parsing behavior depends on the target OS,
        // so to make this test cross-platform we use standard paths
        // or paths that parse as expected on Unix.
        assert_eq!(Shell::from_path("powershell.exe"), Some(Shell::PowerShell));
        assert_eq!(Shell::from_path("/usr/bin/pwsh"), Some(Shell::PowerShell));
        assert_eq!(
            Shell::from_path("powershell_ise.exe"),
            Some(Shell::PowerShell)
        );
    }

    #[test]
    fn test_from_path_elvish() {
        assert_eq!(Shell::from_path("/usr/bin/elvish"), Some(Shell::Elvish));
        assert_eq!(Shell::from_path("elvish"), Some(Shell::Elvish));
    }

    #[test]
    fn test_from_path_nushell() {
        assert_eq!(Shell::from_path("/usr/bin/nu"), Some(Shell::Nushell));
        assert_eq!(Shell::from_path("nu.exe"), Some(Shell::Nushell));
        assert_eq!(Shell::from_path("nushell"), Some(Shell::Nushell));
    }

    #[test]
    fn test_from_path_unknown() {
        assert_eq!(Shell::from_path("/bin/sh"), None);
        assert_eq!(Shell::from_path("cmd.exe"), None);
        assert_eq!(Shell::from_path("python"), None);
        assert_eq!(Shell::from_path("unknown_shell"), None);
    }

    #[test]
    fn test_from_path_empty_and_invalid() {
        assert_eq!(Shell::from_path(""), None);
        assert_eq!(Shell::from_path("/"), None);
        assert_eq!(Shell::from_path("."), None);
        assert_eq!(Shell::from_path(".."), None);

        // .bash file stem is ".bash" which won't match "bash", so it should be None.
        assert_eq!(Shell::from_path(".bash"), None);
    }

    // ================================================================
    // Shell::from_str and Display
    // ================================================================

    #[test]
    fn test_shell_from_str() {
        assert_eq!(
            <Shell as std::str::FromStr>::from_str("bash").unwrap(),
            Shell::Bash
        );
        assert_eq!(
            <Shell as std::str::FromStr>::from_str("zsh").unwrap(),
            Shell::Zsh
        );
        assert_eq!(
            <Shell as std::str::FromStr>::from_str("fish").unwrap(),
            Shell::Fish
        );
        assert_eq!(
            <Shell as std::str::FromStr>::from_str("powershell").unwrap(),
            Shell::PowerShell
        );
        assert_eq!(
            <Shell as std::str::FromStr>::from_str("elvish").unwrap(),
            Shell::Elvish
        );
        assert_eq!(
            <Shell as std::str::FromStr>::from_str("nushell").unwrap(),
            Shell::Nushell
        );
    }

    #[test]
    fn test_shell_from_str_alias() {
        assert_eq!(
            <Shell as std::str::FromStr>::from_str("pwsh").unwrap(),
            Shell::PowerShell
        );
    }

    #[test]
    fn test_shell_from_str_invalid() {
        assert!(<Shell as std::str::FromStr>::from_str("cmd").is_err());
        assert!(<Shell as std::str::FromStr>::from_str("").is_err());
        assert!(<Shell as std::str::FromStr>::from_str("BASH").is_err()); // case-sensitive
    }

    #[test]
    fn test_shell_display() {
        assert_eq!(format!("{}", Shell::Bash), "bash");
        assert_eq!(format!("{}", Shell::Zsh), "zsh");
        assert_eq!(format!("{}", Shell::Fish), "fish");
        assert_eq!(format!("{}", Shell::PowerShell), "powershell");
        assert_eq!(format!("{}", Shell::Elvish), "elvish");
        assert_eq!(format!("{}", Shell::Nushell), "nushell");
    }

    // ================================================================
    // TryFrom conversions: AotShell ↔ Shell ↔ NushellShell
    // ================================================================

    #[test]
    fn test_shell_to_aotshell() {
        let aot: AotShell = Shell::Bash.try_into().unwrap();
        assert_eq!(aot, AotShell::Bash);
        let aot: AotShell = Shell::Zsh.try_into().unwrap();
        assert_eq!(aot, AotShell::Zsh);
        let aot: AotShell = Shell::Fish.try_into().unwrap();
        assert_eq!(aot, AotShell::Fish);
        let aot: AotShell = Shell::PowerShell.try_into().unwrap();
        assert_eq!(aot, AotShell::PowerShell);
        let aot: AotShell = Shell::Elvish.try_into().unwrap();
        assert_eq!(aot, AotShell::Elvish);
    }

    #[test]
    fn test_nushell_to_aotshell_fails() {
        let result: Result<AotShell, _> = Shell::Nushell.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_aotshell_to_shell() {
        let shell: Shell = AotShell::Bash.try_into().unwrap();
        assert_eq!(shell, Shell::Bash);
        let shell: Shell = AotShell::Zsh.try_into().unwrap();
        assert_eq!(shell, Shell::Zsh);
        let shell: Shell = AotShell::Fish.try_into().unwrap();
        assert_eq!(shell, Shell::Fish);
        let shell: Shell = AotShell::PowerShell.try_into().unwrap();
        assert_eq!(shell, Shell::PowerShell);
        let shell: Shell = AotShell::Elvish.try_into().unwrap();
        assert_eq!(shell, Shell::Elvish);
    }

    #[test]
    fn test_shell_to_nushell_shell() {
        // NushellShell doesn't implement PartialEq/Debug, so just check it doesn't panic
        let _nu: NushellShell = Shell::Nushell.try_into().unwrap();
    }

    #[test]
    fn test_non_nushell_to_nushell_shell_fails() {
        let result: Result<NushellShell, _> = Shell::Bash.try_into();
        assert!(result.is_err());
        let result: Result<NushellShell, _> = Shell::Zsh.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_nushell_shell_to_shell() {
        let shell: Shell = NushellShell.try_into().unwrap();
        assert_eq!(shell, Shell::Nushell);
    }

    // ================================================================
    // try_to_clap_complete
    // ================================================================

    #[test]
    fn test_try_to_clap_complete_all_variants() {
        // All shell variants should successfully convert
        assert!(Shell::Bash.try_to_clap_complete().is_ok());
        assert!(Shell::Zsh.try_to_clap_complete().is_ok());
        assert!(Shell::Fish.try_to_clap_complete().is_ok());
        assert!(Shell::PowerShell.try_to_clap_complete().is_ok());
        assert!(Shell::Elvish.try_to_clap_complete().is_ok());
        assert!(Shell::Nushell.try_to_clap_complete().is_ok());
    }

    // ================================================================
    // ValueEnum
    // ================================================================

    #[test]
    fn test_value_variants_count() {
        assert_eq!(Shell::value_variants().len(), 6);
    }

    #[test]
    fn test_to_possible_value_all_some() {
        for variant in Shell::value_variants() {
            assert!(variant.to_possible_value().is_some());
        }
    }

    // ================================================================
    // Generator: file_name and try_generate
    // ================================================================

    #[test]
    fn test_generator_file_name() {
        let bash_name = <Shell as Generator>::file_name(&Shell::Bash, "submod");
        assert!(!bash_name.is_empty());

        let nu_name = <Shell as Generator>::file_name(&Shell::Nushell, "submod");
        assert!(nu_name.contains("submod"));
    }

    #[test]
    fn test_generator_try_generate_produces_output() {
        use clap::Command;

        for shell in Shell::value_variants() {
            let mut cmd = Command::new("test-cmd").subcommand(Command::new("sub1"));
            let mut buf = Vec::new();
            // clap_complete::generate sets bin_name internally
            clap_complete::generate(*shell, &mut cmd, "test-cmd", &mut buf);
            assert!(!buf.is_empty(), "Empty output for {:?}", shell);
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

impl Shell {
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
        shell_self
            .map(|s| s.file_name(name))
            .unwrap_or_else(|_| format!("{name}.nu")) // Default to Nushell if conversion fails
    }

    /// Generates the completion file for the given command and writes it to the provided buffer.
    fn generate(&self, cmd: &clap::Command, buf: &mut dyn std::io::Write) {
        let shell_self = self.try_to_clap_complete();
        shell_self
            .map(|s| {
                s.try_generate(cmd, buf)
                    .unwrap_or_else(|e| panic!("failed to write completion file: {}", e))
            })
            .unwrap_or_else(|_| panic!("failed to write completion file"));
    }

    /// Attempts to generate the completion file for the given command and writes it to the provided buffer.
    fn try_generate(
        &self,
        cmd: &clap::Command,
        buf: &mut dyn std::io::Write,
    ) -> Result<(), std::io::Error> {
        let shell_self = self.try_to_clap_complete();
        match shell_self {
            Ok(s) => s.try_generate(cmd, buf),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}
