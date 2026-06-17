# @oxlint-plugins/core

Shared Rust native core for the [`@oxlint-plugins`](https://github.com/ubugeeei-prod/oxlint-plugins) oxlint plugins.

Every Rust-backed plugin's rule logic is compiled into this **single** NAPI-RS
native addon. The individual `@oxlint-plugins/oxlint-plugin-*` packages are thin
JavaScript facades that depend on this package, so installing several plugins
downloads and links the native code only once instead of once per plugin.

You usually do not depend on this package directly — install a plugin package,
or the [`@oxlint-plugins/oxlint`](../oxlint) bundle, which pull it in for you.

## Shape

The binding exposes one namespace per plugin, so exported names never collide as
more plugins are ported:

```js
const core = require('@oxlint-plugins/core');
core.eslintComments.scanNoUnlimitedDisable(comments);
core.noForbiddenIdentifiers.scanForbiddenIdentifiers(sourceText, options);
core.stylistic.runNativeStylisticLint(sourceText, config);
```

This is unofficial community work and is not an official Oxlint project.
