# Installation

TSN now supports a one-command local installer that sets:

- binary location,
- lsp binary location,
- stdlib location,
- cache directory,
- environment variables.

## Windows (PowerShell)

From repo root:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

This installs to `%USERPROFILE%\.tsn`:

- binary: `%USERPROFILE%\.tsn\bin\tsn.exe`
- lsp: `%USERPROFILE%\.tsn\bin\tsn-lsp.exe`
- stdlib: `%USERPROFILE%\.tsn\stdlib`
- cache: `%USERPROFILE%\.tsn\cache`

It also sets user-level env vars: `TSN_HOME`, `TSN_STDLIB`, `TSN_CACHE_DIR`, and adds TSN bin to `PATH`.

## Linux/macOS

From repo root:

```sh
chmod +x ./scripts/install.sh
./scripts/install.sh
```

This installs to `~/.tsn` and appends env exports to your shell profile (`~/.bashrc` or `~/.zshrc`).
Installed binaries:

- `~/.tsn/bin/tsn`
- `~/.tsn/bin/tsn-lsp`

## Verify installation

```sh
tsn doctor
```

Then run:

```sh
tsn ./examples/production-test.tsn
```
