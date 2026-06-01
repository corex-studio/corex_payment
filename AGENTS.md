# AGENTS.md

## Build & develop

```bash
cargo build
cargo check
cargo clippy
cargo test        # no test suite exists; this will compile only
```

There is no CI, lint-staged, or pre-commit config.

## Architecture

- **`acquiring/`** — payment terminal (эквайринг). Two protocol adapters implementing `Acquiring` trait:
  - `SBAdapter` — Sberbank (SB) + TTK. Spawns external `sb_pilot.exe` from `libs/sc552/`.
  - `InpasAdapter` — Inpas protocol (XML over TCP/USB), requires `dc_host` (Dual Connector proxy).
- **`kkt/`** — cash register / fiscalization (ККТ). Spawns a local `libs/kkt` binary as a child process and talks to it via HTTP on `localhost:3000`.
- The crate is built as both `cdylib` (FFI) and `rlib`.

## Key gotchas

- **`libs/` and `examples/` are gitignored.** They contain platform-native binaries (`kkt`, `sb_pilot.exe`, drivers) and personal test scripts. Do not commit them.
- **No test suite.** Run `cargo build` or `cargo check` for verification.
- **Two distinct `ConnectionType` enums** — one for acquiring (`acquiring::types::ConnectionType`: `Tcp|Usb|Bluetooth`), one for KKT (`kkt::types::ConnectionType`: `Usb|Com|Tcp`). Don't confuse them.
- **Inpas adapter requires `dc_host`** in `ConnectionConfig` or it panics on send.
- **Payment amounts are in minor currency units** (копейки): `10000` = 100 RUB.
- **Error messages are in Russian** (`ProcessError`, healthcheck output).
- **Trait objects use `Box<dyn Acquiring>`** — the protocol adapter is selected at runtime from `ConnectionConfig.protocol`.
- **KKT server process is spawned synchronously** (`std::process::Command`) even though `run_server` is `async`. The healthcheck trait methods mix sync (`check_drivers`) and async (`check_port`).
