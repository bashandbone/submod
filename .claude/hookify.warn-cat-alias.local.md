---
name: warn-cat-alias
enabled: true
event: bash
pattern: ^cat\s+
action: warn
---

**`cat` is aliased to `bat --color=always` on this system.**

Using `cat` will inject ANSI escape codes into output. This corrupts:
- File contents when redirecting (`cat file > other`)
- Git commit messages when used in heredocs
- Any pipeline expecting plain text

**Use instead:**
- `command cat` for plain text output
- The **Read** tool to read files
- The **Write** tool to create files
