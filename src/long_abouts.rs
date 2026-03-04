// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! A series of multiline strings for long-about text. We put them here to keep the command module somewhat readable.

pub const COMPLETE_ME: &str = r#"
Generates shell completion script for the specified shell.

Supported shells:
- `bash`: Bourne Again SHell
- `elvish`: Elvish shell
- `fish`: Friendly Interactive SHell
- `powershell` | `pwsh`: PowerShell
- `zsh`: Z Shell
- `nu` | `nushell`: Nushell

Usage:
    submod completeme [shell] >> /path/to/completion_script

    Examples for common shells and script locations:
    - Bash: `submod completeme bash > ~/.bash_completion.d/submod` or `submod completeme bash > ~/.config/bash_completion/submod`

    - Elvish: `submod completeme elvish > ~/.config/elvish/completions/submod.elv`

    - Fish: `submod completeme fish > ~/.config/fish/completions/submod.fish`

    - PowerShell: `submod completeme powershell > ~/.config/powershell/completions/submod.ps1` or `submod completeme pwsh > ~/.config/powershell/completions/submod.ps1`

    - Zsh: `submod completeme zsh > ~/.zsh/completions/_submod` or `submod completeme zsh > ~/.zfunc/_submod`

    - Nushell: `submod completeme nu > "$NUSHELL_CONFIG_DIR/scripts/completions/submod.nu" && echo 'use completions/submod.nu' >> "$NU_CONFIG_PATH"`
"#;
