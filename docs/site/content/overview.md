# oxlint-plugins

Rust-backed Oxlint plugin packages for porting the ESLint ecosystem.

The current public package surface is Oxlint's JavaScript plugin API. The performance-sensitive rule core is implemented in Rust and exposed through NAPI-RS. Each plugin package stays independently installable and publishable.

This is unofficial community work, not an official Oxlint project.

## Current Shape

- Vite+ drives build, lint, format, test, and release tasks.
- Root `vp build` runs `cargo build --workspace --all-features` before package builds.
- Blacksmith 32 vCPU runners verify CI.
- npm trusted publishing handles release without long-lived tokens.
- pnpm v11 is the package manager.
- TypeScript checks use `tsgo`, not `tsc`.
- `crates/_carton` provides shared allocation, profiling, and fast-hash primitives.
- Rust rule logic is grouped by domain crates such as `crates/stylistic`.
- `status.json` tracks package and rule status.

## Docs Stack

This site renders Markdown with `@ox-content/wasm` and is configured with the Void SDK through `voidPlugin()`.
