// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! A series of multiline strings for long-about text. We put them here to keep the command module somewhat readable.
pub const COMPLETE_ME: &str = r"
Generates a shell completion script from the current command model on stdout.
Save the output and load it using your shell's completion configuration.

Supported shells:
- `bash`: Bourne Again SHell
- `elvish`: Elvish shell
- `fish`: Friendly Interactive SHell
- `powershell` | `pwsh`: PowerShell
- `zsh`: Z Shell
- `nu` | `nushell`: Nushell

Usage:
    submod completeme <shell> > /path/to/completion_script

    Examples for common shells and script locations:
    - Bash: `submod completeme bash > ~/.bash_completion.d/submod` or `submod completeme bash > ~/.config/bash_completion/submod`

    - Elvish: `submod completeme elvish > ~/.config/elvish/completions/submod.elv`

    - Fish: `submod completeme fish > ~/.config/fish/completions/submod.fish`

    - PowerShell: `submod completeme powershell > $Home\Documents\PowerShell\completions\submod.completion.ps1`
    
    - Zsh: `submod completeme zsh > ~/.zsh/completions/_submod` or `submod completeme zsh > ~/.zfunc/_submod`

    - Nushell: `submod completeme nu` (save stdout as submod.nu and load it from your Nushell configuration)
";
