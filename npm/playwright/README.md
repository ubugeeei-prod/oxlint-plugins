# @oxlint-plugins/oxlint-plugin-playwright

Rust-backed Oxlint plugin port of `eslint-plugin-playwright` v2.10.4.

This package exposes the `playwright` plugin, recommended configs, and a native
API for scanning Playwright test source through the Rust implementation.

## Restricted rule options

The option-bearing `no-restricted-locators`, `no-restricted-matchers`, and
`no-restricted-roles` rules follow the pinned v2.10.4 contract. Locator and role
lists accept either strings or objects with an optional custom message, while
matcher restrictions use a matcher-chain-to-message map:

```jsonc
{
  "rules": {
    "playwright/no-restricted-locators": [
      "error",
      [
        "getByTestId",
        {
          "type": "getByTitle",
          "message": "Prefer accessible locators",
        },
      ],
    ],
    "playwright/no-restricted-matchers": [
      "error",
      {
        "not.toBeTruthy": "Prefer a positive matcher",
      },
    ],
    "playwright/no-restricted-roles": [
      "error",
      [
        "progressbar",
        {
          "role": "alert",
          "message": "Assert on specific content",
        },
      ],
    ],
  },
}
```

The direct API accepts the same option values:

```js
const { scanPlaywright } = require('@oxlint-plugins/oxlint-plugin-playwright/api');

scanPlaywright('page.getByTestId("submit")', 'fixture.spec.ts', {
  noRestrictedLocators: ['getByTestId'],
});
```

Static dot, string-computed, and template-computed member names are supported.
Matcher modifiers and chains are checked against Playwright `expect`, configured
global aliases, and named `expect` import aliases.
