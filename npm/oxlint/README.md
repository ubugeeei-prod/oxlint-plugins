# @oxlint-plugins/oxlint

Convenience bundle for the [`@oxlint-plugins`](https://github.com/ubugeeei-prod/oxlint-plugins) suite.

Installing this one package pulls in the shared native core
([`@oxlint-plugins/core`](../core)) and every plugin facade, and exposes a single
combined oxlint plugin so you can register the whole suite with one `jsPlugins`
entry. The native code is compiled once into the shared core, so adding more
plugins does not add more binaries.

## Usage

```jsonc
// oxlint.config.jsonc
{
  "jsPlugins": [{ "name": "oxlint", "specifier": "@oxlint-plugins/oxlint" }],
  "rules": {
    "oxlint/no-unlimited-disable": "error",
    "oxlint/quotes": ["error", "single"],
  },
}
```

Rules keep their upstream names under the `oxlint/` namespace.

### Per-plugin namespaces

If you prefer each plugin's own namespace (matching the upstream ESLint plugin
rule IDs), depend on the individual `@oxlint-plugins/oxlint-plugin-*` packages
directly. They are also re-exported here as `require('@oxlint-plugins/oxlint').plugins`.

This is unofficial community work and is not an official Oxlint project.
