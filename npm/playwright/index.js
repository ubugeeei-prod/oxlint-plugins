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
const optionRequiredRules = new Set([
  'no-restricted-locators',
  'no-restricted-matchers',
  'no-restricted-roles',
]);

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
  return {
    meta: {
      type: ruleType(ruleName),
      docs: {
        description: `enforce playwright ${ruleName.replaceAll('-', ' ')}`,
        category: 'Best Practices',
        recommended: recommendedRuleConfig[`playwright/${ruleName}`] !== undefined,
        url: `${DOCS_BASE}/${ruleName}.md`,
      },
      fixable: fixableRule(ruleName) ? 'code' : undefined,
      messages: {
        unexpected: 'Unexpected Playwright pattern.',
      },
      // The core does not honor upstream option values (it has no options
      // struct). Declare no schema so configured options surface as an error
      // rather than being silently dropped. The `no-restricted-*` rules need
      // real options support to function; tracked for implementation.
      schema: [],
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
    ruleName.startsWith('prefer-') ||
    ruleName === 'require-to-pass-timeout' ||
    ruleName === 'require-to-throw-message'
  );
}

function diagnosticsForRule(context, ruleName) {
  if (optionRequiredRules.has(ruleName) && !hasConfiguredOptions(context.options)) {
    return [];
  }

  return diagnosticsForContext(context).filter((diagnostic) => diagnostic.ruleName === ruleName);
}

function diagnosticsForContext(context) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.spec.ts';
  let cached = diagnosticsCache.get(sourceCode);

  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const diagnostics = scanPlaywright(sourceText, filename);
  cached = { sourceText, filename, diagnostics };
  diagnosticsCache.set(sourceCode, cached);
  return diagnostics;
}

function reportDiagnostic(context, diagnostic) {
  context.report({
    messageId: diagnostic.messageId,
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
  });
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

function hasConfiguredOptions(options) {
  return (
    Array.isArray(options) &&
    options.some((option) => {
      if (Array.isArray(option)) return option.length > 0;
      return option && typeof option === 'object' && Object.keys(option).length > 0;
    })
  );
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedPlaywrightRuleNames = implementedRuleNames;
module.exports.scanPlaywright = scanPlaywright;
