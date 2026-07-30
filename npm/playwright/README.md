# @oxlint-plugins/oxlint-plugin-playwright

Rust-backed Oxlint plugin port of `eslint-plugin-playwright` v2.10.4.

This package exposes the `playwright` plugin, recommended configs, and a native
API for scanning Playwright test source through the Rust implementation.

## Assertion function options

`expect-expect` accepts the complete upstream v2.10.4 assertion-function
contract:

```json
{
  "rules": {
    "playwright/expect-expect": [
      "error",
      {
        "assertFunctionNames": ["assertCustomCondition"],
        "assertFunctionPatterns": ["^assert.*", "^verify.*"]
      }
    ]
  }
}
```

Exact names match direct calls and terminal member identifiers such as
`page.assertCustomCondition()`. Patterns use JavaScript regular-expression
syntax. Assertions inside nested callbacks and `test.step()` count toward their
containing test, matching upstream ancestry behavior.

## Numeric threshold options

The numeric threshold rules implement the complete upstream v2.10.4 option
contracts:

```json
{
  "rules": {
    "playwright/max-expects": ["error", { "max": 5 }],
    "playwright/max-nested-describe": ["error", { "max": 5 }],
    "playwright/require-top-level-describe": ["error", { "maxTopLevelDescribes": 2 }]
  }
}
```

`max-expects` counts Playwright assertion calls per test callback while
preserving upstream nested-callback reset behavior. `max-nested-describe`
measures actual AST nesting, and `require-top-level-describe` rejects top-level
tests and hooks with an optional top-level describe limit. Imported aliases,
`test.extend()` aliases, and `settings.playwright.globalAliases` are supported.

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
  assertFunctionNames: ['assertCustomCondition'],
  assertFunctionPatterns: ['^verify.*'],
  maxExpects: 5,
  maxNestedDescribe: 5,
  maxTopLevelDescribes: 2,
  testAliases: ['it'],
  expectAliases: ['verify'],
  noRestrictedLocators: ['getByTestId'],
});
```

Static dot, string-computed, and template-computed member names are supported.
Matcher modifiers and chains are checked against Playwright `expect`, configured
global aliases, and named `expect` import aliases.

## Title and tag options

`valid-title` supports the complete v2.10.4 option family: disallowed words,
space handling, per-call-kind type checks, and `mustMatch` / `mustNotMatch`
patterns with optional custom messages. `valid-test-tags` supports mutually
exclusive allow and deny lists containing exact strings or regular expressions:

```jsonc
{
  "rules": {
    "playwright/valid-title": [
      "error",
      {
        "disallowedWords": ["correct", "properly"],
        "mustMatch": {
          "describe": ["#(?:unit|e2e)", "Describe titles need a test kind"],
          "test": "#(?:unit|e2e)",
        },
      },
    ],
    "playwright/valid-test-tags": [
      "error",
      {
        "allowedTags": ["@smoke", { "source": "^@team-" }],
      },
    ],
  },
}
```

The direct API uses `validTitle` and `validTestTags` with the same values.
`testAliases` corresponds to `settings.playwright.globalAliases.test`. Imported
`test` aliases and test functions created with `test.extend()` are discovered
automatically.

Title fixes preserve the original quote kind, return UTF-16 ranges to JavaScript,
and converge across repeated fix passes. The pinned fixture replays all 247
authored upstream cases for these rules (116 diagnostics and 39 fix-output
contracts) in both the direct API and the Oxlint plugin adapter.
