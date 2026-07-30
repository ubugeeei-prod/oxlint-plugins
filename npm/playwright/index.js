'use strict';

// Oxlint plugin port of eslint-plugin-playwright (MIT).
// The JavaScript layer is an Oxlint/NAPI adapter; Playwright rule scans run in
// Rust through Oxc-backed source parsing and fast structural pattern checks.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedPlaywrightRuleNames, scanPlaywright } = require('./api.js');

const PLUGIN_NAME = 'playwright';
const DOCS_BASE = 'https://github.com/mskelton/eslint-plugin-playwright/blob/main/docs/rules';
const diagnosticsCache = new WeakMap();
const implementedRuleNames = Object.freeze(implementedPlaywrightRuleNames());
const restrictedRules = new Set([
  'no-restricted-locators',
  'no-restricted-matchers',
  'no-restricted-roles',
]);
const patternRules = new Set(['valid-test-tags', 'valid-title']);
const thresholdRules = new Set([
  'max-expects',
  'max-nested-describe',
  'require-top-level-describe',
]);
const optionAwareTitleRules = new Set(['prefer-lowercase-title']);

const sharedGlobals = Object.freeze({
  expect: false,
  test: false,
});

const recommendedRuleConfig = Object.freeze({
  'no-empty-pattern': 'off',
  'playwright/consistent-spacing-between-blocks': 'warn',
  'playwright/expect-expect': 'warn',
  'playwright/max-nested-describe': 'warn',
  'playwright/missing-playwright-await': 'error',
  'playwright/no-conditional-expect': 'warn',
  'playwright/no-conditional-in-test': 'warn',
  'playwright/no-duplicate-hooks': 'warn',
  'playwright/no-duplicate-slow': 'warn',
  'playwright/no-element-handle': 'warn',
  'playwright/no-eval': 'warn',
  'playwright/no-focused-test': 'error',
  'playwright/no-force-option': 'warn',
  'playwright/no-nested-step': 'warn',
  'playwright/no-networkidle': 'error',
  'playwright/no-page-pause': 'warn',
  'playwright/no-skipped-test': 'warn',
  'playwright/no-standalone-expect': 'error',
  'playwright/no-unsafe-references': 'error',
  'playwright/no-unused-locators': 'error',
  'playwright/no-useless-await': 'warn',
  'playwright/no-useless-not': 'warn',
  'playwright/no-wait-for-navigation': 'error',
  'playwright/no-wait-for-selector': 'warn',
  'playwright/no-wait-for-timeout': 'warn',
  'playwright/prefer-hooks-in-order': 'warn',
  'playwright/prefer-hooks-on-top': 'warn',
  'playwright/prefer-locator': 'warn',
  'playwright/prefer-to-have-count': 'warn',
  'playwright/prefer-to-have-length': 'warn',
  'playwright/prefer-web-first-assertions': 'error',
  'playwright/valid-describe-callback': 'error',
  'playwright/valid-expect': 'error',
  'playwright/valid-expect-in-promise': 'error',
  'playwright/valid-test-tags': 'error',
  'playwright/valid-title': 'error',
});

const layoutRules = new Set(['consistent-spacing-between-blocks']);
const problemRules = new Set([
  'expect-expect',
  'missing-playwright-await',
  'no-commented-out-tests',
  'no-conditional-expect',
  'no-conditional-in-test',
  'no-duplicate-slow',
  'no-element-handle',
  'no-eval',
  'no-focused-test',
  'no-force-option',
  'no-nested-step',
  'no-networkidle',
  'no-nth-methods',
  'no-page-pause',
  'no-skipped-test',
  'no-standalone-expect',
  'no-unsafe-references',
  'no-unused-locators',
  'no-useless-await',
  'no-useless-not',
  'valid-describe-callback',
  'valid-expect',
  'valid-test-tags',
]);

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createPlaywrightRule(ruleName)]),
  ),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 0])),
  configs: {
    'flat/recommended': createFlatRecommendedConfig(),
    'playwright-test': createLegacyRecommendedConfig(),
    recommended: createLegacyRecommendedConfig(),
  },
});

plugin.implementedPlaywrightRuleNames = implementedRuleNames;
plugin.scanPlaywright = scanPlaywright;

function createFlatRecommendedConfig() {
  return {
    name: 'playwright/flat/recommended',
    languageOptions: {
      globals: sharedGlobals,
    },
    plugins: [PLUGIN_NAME],
    rules: recommendedRuleConfig,
  };
}

function createLegacyRecommendedConfig() {
  return {
    env: {
      'shared-node-browser': true,
    },
    plugins: [PLUGIN_NAME],
    rules: recommendedRuleConfig,
  };
}

function createPlaywrightRule(ruleName) {
  const specializedMeta =
    expectExpectRuleMeta(ruleName) ??
    preferLowercaseTitleRuleMeta(ruleName) ??
    restrictedRuleMeta(ruleName) ??
    patternRuleMeta(ruleName) ??
    thresholdRuleMeta(ruleName);
  return {
    meta: {
      type: ruleType(ruleName),
      docs: {
        description:
          specializedMeta?.description ?? `enforce playwright ${ruleName.replaceAll('-', ' ')}`,
        category: 'Best Practices',
        recommended: recommendedRuleConfig[`playwright/${ruleName}`] !== undefined,
        url: specializedMeta?.url ?? `${DOCS_BASE}/${ruleName}.md`,
      },
      fixable: fixableRule(ruleName) ? 'code' : undefined,
      messages: specializedMeta?.messages ?? {
        unexpected: 'Unexpected Playwright pattern.',
      },
      schema: specializedMeta?.schema ?? [],
    },
    createOnce(context) {
      return {
        Program() {
          for (const diagnostic of diagnosticsForRule(context, ruleName)) {
            reportDiagnostic(context, diagnostic);
          }
        },
      };
    },
  };
}

function ruleType(ruleName) {
  if (layoutRules.has(ruleName)) return 'layout';
  if (problemRules.has(ruleName)) return 'problem';
  return 'suggestion';
}

function fixableRule(ruleName) {
  return (
    ruleName === 'no-focused-test' ||
    ruleName === 'no-skipped-test' ||
    ruleName === 'no-slowed-test' ||
    ruleName === 'valid-title' ||
    ruleName.startsWith('prefer-') ||
    ruleName === 'require-to-pass-timeout' ||
    ruleName === 'require-to-throw-message'
  );
}

function diagnosticsForRule(context, ruleName) {
  const options =
    ruleName === 'expect-expect'
      ? expectExpectScanOptions(context)
      : restrictedRules.has(ruleName) || patternRules.has(ruleName)
        ? ruleScanOptions(context, ruleName)
        : thresholdRules.has(ruleName)
          ? thresholdScanOptions(context, ruleName)
          : optionAwareTitleRules.has(ruleName)
            ? preferLowercaseTitleScanOptions(context)
            : undefined;
  return diagnosticsForContext(context, options).filter(
    (diagnostic) => diagnostic.ruleName === ruleName,
  );
}

function diagnosticsForContext(context, options) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.spec.ts';
  const optionsKey = JSON.stringify(options ?? null, (_key, value) =>
    value instanceof RegExp ? { source: value.source, flags: value.flags } : value,
  );
  let cached = diagnosticsCache.get(sourceCode);

  if (!cached || cached.sourceText !== sourceText || cached.filename !== filename) {
    cached = { sourceText, filename, diagnosticsByOptions: new Map() };
    diagnosticsCache.set(sourceCode, cached);
  }
  const existing = cached.diagnosticsByOptions.get(optionsKey);
  if (existing) {
    return existing;
  }

  const diagnostics = scanPlaywright(sourceText, filename, options);
  cached.diagnosticsByOptions.set(optionsKey, diagnostics);
  return diagnostics;
}

function reportDiagnostic(context, diagnostic) {
  const descriptor = {
    messageId: diagnostic.messageId,
    data: diagnostic.data,
    loc: {
      start: {
        line: diagnostic.loc.startLine,
        column: diagnostic.loc.startColumn,
      },
      end: {
        line: diagnostic.loc.endLine,
        column: diagnostic.loc.endColumn,
      },
    },
  };
  if (diagnostic.fix) {
    descriptor.fix = (fixer) =>
      fixer.replaceTextRange(
        [diagnostic.fix.start, diagnostic.fix.end],
        diagnostic.fix.replacement,
      );
  }
  context.report(descriptor);
}

function sourceTextForContext(context) {
  const sourceCode = context.sourceCode || {};
  if (typeof sourceCode.getText === 'function') {
    return sourceCode.getText();
  }
  if (typeof sourceCode.text === 'string') {
    return sourceCode.text;
  }
  return '';
}

function expectExpectScanOptions(context) {
  const options = Array.isArray(context.options) ? context.options : [];
  const configured = options[0] && typeof options[0] === 'object' ? options[0] : {};
  const expectAliases = context.settings?.playwright?.globalAliases?.expect;
  const testAliases = context.settings?.playwright?.globalAliases?.test;
  return {
    assertFunctionNames: configured.assertFunctionNames,
    assertFunctionPatterns: configured.assertFunctionPatterns,
    ...(Array.isArray(expectAliases) ? { expectAliases } : {}),
    ...(Array.isArray(testAliases) ? { testAliases } : {}),
  };
}

function preferLowercaseTitleScanOptions(context) {
  const options = Array.isArray(context.options) ? context.options : [];
  const configured = options[0] && typeof options[0] === 'object' ? options[0] : {};
  const testAliases = context.settings?.playwright?.globalAliases?.test;
  return {
    allowedPrefixes: configured.allowedPrefixes,
    ignore: configured.ignore,
    ignoreTopLevelDescribe: configured.ignoreTopLevelDescribe,
    ...(Array.isArray(testAliases) ? { testAliases } : {}),
  };
}

function thresholdScanOptions(context, ruleName) {
  const options = Array.isArray(context.options) ? context.options : [];
  const configured = options[0] && typeof options[0] === 'object' ? options[0] : {};
  const expectAliases = context.settings?.playwright?.globalAliases?.expect;
  const testAliases = context.settings?.playwright?.globalAliases?.test;
  return {
    ...(ruleName === 'max-expects' ? { maxExpects: configured.max } : {}),
    ...(ruleName === 'max-nested-describe' ? { maxNestedDescribe: configured.max } : {}),
    ...(ruleName === 'require-top-level-describe'
      ? { maxTopLevelDescribes: configured.maxTopLevelDescribes }
      : {}),
    ...(Array.isArray(expectAliases) ? { expectAliases } : {}),
    ...(Array.isArray(testAliases) ? { testAliases } : {}),
  };
}

function ruleScanOptions(context, ruleName) {
  const options = Array.isArray(context.options) ? context.options : [];
  const expectAliases = context.settings?.playwright?.globalAliases?.expect;
  const testAliases = context.settings?.playwright?.globalAliases?.test;
  return {
    ...(ruleName === 'no-restricted-locators' ? { noRestrictedLocators: options[0] } : {}),
    ...(ruleName === 'no-restricted-matchers' ? { noRestrictedMatchers: options[0] } : {}),
    ...(ruleName === 'no-restricted-roles' ? { noRestrictedRoles: options[0] } : {}),
    ...(ruleName === 'valid-title' ? { validTitle: options[0] } : {}),
    ...(ruleName === 'valid-test-tags' ? { validTestTags: options[0] } : {}),
    ...(Array.isArray(expectAliases) ? { expectAliases } : {}),
    ...(Array.isArray(testAliases) ? { testAliases } : {}),
  };
}

function patternRuleMeta(ruleName) {
  if (ruleName === 'valid-test-tags') {
    return {
      description: 'Enforce valid tag format in Playwright test blocks and titles',
      messages: {
        disallowedTag: 'Tag "{{tag}}" is not allowed',
        invalidTagFormat: 'Tag must start with @',
        invalidTagValue: 'Tag must be a string or array of strings',
        unknownTag: 'Unknown tag "{{tag}}"',
      },
      schema: [
        {
          additionalProperties: false,
          properties: {
            allowedTags: tagListSchema(),
            disallowedTags: tagListSchema(),
          },
          type: 'object',
        },
      ],
    };
  }
  if (ruleName === 'valid-title') {
    const matcherAndMessage = {
      additionalItems: false,
      items: { type: 'string' },
      maxItems: 2,
      minItems: 1,
      type: 'array',
    };
    return {
      description: 'Enforce valid titles',
      messages: {
        accidentalSpace: 'should not have leading or trailing spaces',
        disallowedWord: '"{{ word }}" is not allowed in test titles',
        duplicatePrefix: 'should not have duplicate prefix',
        emptyTitle: '{{ functionName }} should not have an empty title',
        mustMatch: '{{ functionName }} should match {{ pattern }}',
        mustMatchCustom: '{{ message }}',
        mustNotMatch: '{{ functionName }} should not match {{ pattern }}',
        mustNotMatchCustom: '{{ message }}',
        titleMustBeString: 'Title must be a string',
      },
      schema: [
        {
          additionalProperties: false,
          patternProperties: {
            '^must(?:Not)?Match$': {
              oneOf: [
                { type: 'string' },
                matcherAndMessage,
                {
                  additionalProperties: {
                    oneOf: [{ type: 'string' }, matcherAndMessage],
                  },
                  propertyNames: { enum: ['describe', 'test', 'step'] },
                  type: 'object',
                },
              ],
            },
          },
          properties: {
            disallowedWords: { items: { type: 'string' }, type: 'array' },
            ignoreSpaces: { default: false, type: 'boolean' },
            ignoreTypeOfDescribeName: { default: false, type: 'boolean' },
            ignoreTypeOfStepName: { default: true, type: 'boolean' },
            ignoreTypeOfTestName: { default: false, type: 'boolean' },
          },
          type: 'object',
        },
      ],
    };
  }
  return null;
}

function tagListSchema() {
  return {
    items: {
      oneOf: [
        { type: 'string' },
        {
          additionalProperties: false,
          properties: { source: { type: 'string' } },
          type: 'object',
        },
      ],
    },
    type: 'array',
  };
}

function expectExpectRuleMeta(ruleName) {
  if (ruleName !== 'expect-expect') {
    return null;
  }
  return {
    description: 'Enforce assertion to be made in a test body',
    messages: {
      noAssertions: 'Test has no assertions',
    },
    schema: [
      {
        additionalProperties: false,
        properties: {
          assertFunctionNames: {
            items: [{ type: 'string' }],
            type: 'array',
          },
          assertFunctionPatterns: {
            items: [{ type: 'string' }],
            type: 'array',
          },
        },
        type: 'object',
      },
    ],
    url: 'https://github.com/mskelton/eslint-plugin-playwright/tree/main/docs/rules/expect-expect.md',
  };
}

function preferLowercaseTitleRuleMeta(ruleName) {
  if (ruleName !== 'prefer-lowercase-title') {
    return null;
  }
  return {
    description: 'Enforce lowercase test names',
    messages: {
      unexpectedLowercase: '`{{method}}`s should begin with lowercase',
    },
    schema: [
      {
        additionalProperties: false,
        properties: {
          allowedPrefixes: {
            additionalItems: false,
            items: { type: 'string' },
            type: 'array',
          },
          ignore: {
            additionalItems: false,
            items: {
              enum: ['test.describe', 'test'],
            },
            type: 'array',
          },
          ignoreTopLevelDescribe: {
            default: false,
            type: 'boolean',
          },
        },
        type: 'object',
      },
    ],
    url: 'https://github.com/mskelton/eslint-plugin-playwright/tree/main/docs/rules/prefer-lowercase-title.md',
  };
}

function thresholdRuleMeta(ruleName) {
  switch (ruleName) {
    case 'max-expects':
      return {
        description: 'Enforces a maximum number assertion calls in a test body',
        messages: {
          exceededMaxAssertion:
            'Too many assertion calls ({{ count }}) - maximum allowed is {{ max }}',
        },
        schema: [maximumSchema('max', 1, 'integer')],
      };
    case 'max-nested-describe':
      return {
        description: 'Enforces a maximum depth to nested describe calls',
        messages: {
          exceededMaxDepth:
            'Maximum describe call depth exceeded ({{ depth }}). Maximum allowed is {{ max }}.',
        },
        schema: [maximumSchema('max', 0, 'integer')],
      };
    case 'require-top-level-describe':
      return {
        description: 'Require test cases and hooks to be inside a `test.describe` block',
        messages: {
          tooManyDescribes:
            'There should not be more than {{amount}} describe{{s}} at the top level',
          unexpectedHook: 'All hooks must be wrapped in a describe block.',
          unexpectedTest: 'All test cases must be wrapped in a describe block.',
        },
        schema: [maximumSchema('maxTopLevelDescribes', 1, 'number')],
      };
    default:
      return null;
  }
}

function maximumSchema(property, minimum, type) {
  return {
    additionalProperties: false,
    properties: {
      [property]: {
        minimum,
        type,
      },
    },
    type: 'object',
  };
}

function restrictedRuleMeta(ruleName) {
  switch (ruleName) {
    case 'no-restricted-locators':
      return {
        description: 'Disallows the usage of specific locator methods',
        messages: {
          restricted: 'Usage of `{{method}}` is disallowed',
          restrictedWithMessage: '{{message}}',
        },
        schema: [restrictedListSchema('type')],
      };
    case 'no-restricted-matchers':
      return {
        description: 'Disallow specific matchers & modifiers',
        messages: {
          restricted: 'Use of `{{restriction}}` is disallowed',
          restrictedWithMessage: '{{message}}',
        },
        schema: [
          {
            additionalProperties: {
              type: ['string', 'null'],
            },
            type: 'object',
          },
        ],
      };
    case 'no-restricted-roles':
      return {
        description: 'Disallows the usage of specific roles in getByRole()',
        messages: {
          restricted: 'Usage of role `{{role}}` in getByRole() is disallowed',
          restrictedWithMessage: '{{message}}',
        },
        schema: [restrictedListSchema('role')],
      };
    default:
      return null;
  }
}

function restrictedListSchema(requiredProperty) {
  return {
    items: {
      oneOf: [
        { type: 'string' },
        {
          additionalProperties: false,
          properties: {
            message: { type: 'string' },
            [requiredProperty]: { type: 'string' },
          },
          required: [requiredProperty],
          type: 'object',
        },
      ],
    },
    type: 'array',
  };
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedPlaywrightRuleNames = implementedRuleNames;
module.exports.scanPlaywright = scanPlaywright;
