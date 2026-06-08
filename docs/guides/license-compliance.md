# License Compliance

This workspace is MIT licensed, but ports must respect the licenses of the existing ecosystem.

## Upstream ESLint Plugins

Before porting a rule, record:

- Upstream package name and version.
- Upstream repository URL.
- Upstream license.
- Whether source, tests, docs, fixtures, or messages are copied, adapted, or only used for behavioral reference.

Do not copy upstream implementation code unless the license permits it and attribution is preserved. Prefer clean-room implementations based on documented behavior and independently written tests.

When copying upstream test cases or fixtures:

- Keep only the minimum needed examples.
- Add source attribution in comments.
- Preserve required license notices.
- Do not copy large docs or source files into this repository.

## Dependency Policy

Allowed dependency licenses are intentionally conservative:

- MIT
- Apache-2.0
- BSD-2-Clause
- BSD-3-Clause
- ISC
- 0BSD
- Zlib
- Unicode-3.0
- MPL-2.0

Anything else requires an explicit entry in the relevant allowlist and a short reason in the review.

The Node dependency check understands simple SPDX `OR` and `AND` expressions:

- `OR` is accepted when at least one branch is entirely allowed.
- `AND` is accepted only when every listed license is allowed.
- Missing metadata, LGPL, GPL, and AGPL require an explicit entry in `tools/license-exceptions.json`.

Exceptions are intended to be narrow and reviewable. The initial exceptions are limited to private documentation tooling and optional prebuilt image dependencies that are not shipped in published plugin packages.

## Release Artifacts

Published npm packages must include:

- Correct `license` metadata.
- Repository metadata pointing back to this repository.
- Generated NAPI artifacts only from CI.
- npm provenance through trusted publishing.
