# Environment

Use Nix for local environment setup.

```sh
nix develop
vp install
vp build
vp test
```

Root `vp build` runs `cargo build --workspace --all-features` before building the independently publishable npm packages.

The dev shell provides:

- Node.js 24
- Rust 1.96.0 with clippy and rustfmt
- `cargo-deny`
- `cargo-insta`
- OpenSSL and pkg-config for native builds
- A `vp` wrapper that installs and runs the native Vite+ CLI

Native Vite+ can also be installed directly:

```sh
curl -fsSL https://vite.plus | bash
```

This workspace uses pnpm v11 through `packageManager`.

If `direnv` is installed:

```sh
direnv allow
```
