# @oxlint-plugins/oxlint-plugin-security

Rust-backed Oxlint plugin port of `eslint-plugin-security` v4.0.0.

The JavaScript layer is an Oxlint/NAPI adapter. Parsing, import/require tracking,
static-expression classification, and rule checks run in Rust through Oxc.

## Rules

- `security/detect-bidi-characters`
- `security/detect-buffer-noassert`
- `security/detect-child-process`
- `security/detect-disable-mustache-escape`
- `security/detect-eval-with-expression`
- `security/detect-new-buffer`
- `security/detect-no-csrf-before-method-override`
- `security/detect-non-literal-fs-filename`
- `security/detect-non-literal-regexp`
- `security/detect-non-literal-require`
- `security/detect-object-injection`
- `security/detect-possible-timing-attacks`
- `security/detect-pseudoRandomBytes`
- `security/detect-unsafe-regex`
