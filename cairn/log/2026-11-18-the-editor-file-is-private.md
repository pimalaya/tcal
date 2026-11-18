---
cairn: log
change: the-editor-file-is-private
landed: 2026-11-18
---

# The editor file is private

The temporary file tCal hands the editor (`tcal-<uuid>.toml`) is created through pimalaya-cli 0.6.1's `file::private_options`: new, 0600 on Unix whatever the umask, a file or a symbolic link already at its path refused. It was written with `fs::write`, readable by every user under a 022 umask, in a directory every user shares. The twin of tCard's change of the same name (tcard d6e4c64), and of calendula's composer draft (its CAL-13) the same day. Test `the_seed_file_is_private_and_new`. Capability **editor** moved.
