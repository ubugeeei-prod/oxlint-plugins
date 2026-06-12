'use strict';

// Oxlint plugin port of eslint-plugin-mocha (MIT).
// The JavaScript layer is only an Oxlint/NAPI adapter; parsing, Mocha call
// classification, suite tracking, and rule checks run in Rust through Oxc.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedMochaRuleNames, scanMocha } = require('./api.js');

const PLUGIN_NAME = 'mocha';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/mocha';
const diagnosticsCache = new WeakMap();

const commonGlobals = Object.freeze({
  after: false,
  afterEach: false,
  before: false,
  beforeEach: false,
  context: false,
  describe: false,
  it: false,
  setup: false,
  specify: false,
  suite: false,
  suiteSetup: false,
  suiteTeardown: false,
  teardown: false,
  test: false,
  xcontext: false,
  xdescribe: false,
  xit: false,
  xspecify: false,
});

const allRuleConfig = Object.freeze({
  'handle-done-callback': 'error',
  'max-top-level-suites': 'error',
  'no-async-suite': 'error',
  'no-exclusive-tests': 'error',
  'no-exports': 'error',
  'no-global-tests': 'error',
  'no-hooks': 'error',
  'no-hooks-for-single-case': 'error',
  'no-identical-title': 'error',
  'no-mocha-arrows': 'error',
  'no-nested-tests': 'error',
  'no-pending-tests': 'error',
  'no-return-and-callback': 'error',
  'no-return-from-async': 'error',
  'no-setup-in-describe': 'error',
  'no-sibling-hooks': 'error',
  'no-synchronous-tests': 'error',
  'no-top-level-hooks': 'error',
  'prefer-arrow-callback': 'error',
  'consistent-spacing-between-blocks': 'error',
  'consistent-interface': ['error', { interface: 'BDD' }],
  'valid-suite-title': 'error',
  'valid-test-title': 'error',
  'no-empty-title': 'error',
});

const recommendedRuleConfig = Object.freeze({
  'handle-done-callback': 'error',
  'max-top-level-suites': ['error', { limit: 1 }],
  'no-async-suite': 'error',
  'no-exclusive-tests': 'warn',
  'no-exports': 'error',
  'no-global-tests': 'error',
  'no-hooks': 'off',
  'no-hooks-for-single-case': 'off',
  'no-identical-title': 'error',
  'no-mocha-arrows': 'error',
  'no-nested-tests': 'error',
  'no-pending-tests': 'warn',
  'no-return-and-callback': 'error',
  'no-return-from-async': 'off',
  'no-setup-in-describe': 'error',
  'no-sibling-hooks': 'error',
  'no-synchronous-tests': 'off',
  'no-top-level-hooks': 'warn',
  'prefer-arrow-callback': 'off',
  'valid-suite-title': 'off',
  'valid-test-title': 'off',
  'no-empty-title': 'error',
  'consistent-spacing-between-blocks': 'error',
});

const ruleDescriptions = Object.freeze({
  'consistent-interface': 'enforce consistent use of Mocha interfaces',
  'consistent-spacing-between-blocks': 'require consistent spacing between Mocha blocks',
  'handle-done-callback': 'require callback-based async tests to handle the callback',
  'max-top-level-suites': 'enforce a maximum number of top-level suites',
  'no-async-suite': 'disallow async functions passed to suites',
  'no-empty-title': 'disallow empty test descriptions',
  'no-exclusive-tests': 'disallow exclusive Mocha tests',
  'no-exports': 'disallow exports from test files',
  'no-global-tests': 'disallow global tests',
  'no-hooks': 'disallow hooks',
  'no-hooks-for-single-case': 'disallow hooks for suites with a single test',
  'no-identical-title': 'disallow duplicate suite or test titles in the same scope',
  'no-mocha-arrows': 'disallow arrow callbacks in Mocha tests',
  'no-nested-tests': 'disallow tests nested inside tests',
  'no-pending-tests': 'disallow pending tests',
  'no-return-and-callback': 'disallow returning in a test or hook that uses a callback',
  'no-return-from-async': 'disallow returning from async tests or hooks',
  'no-setup-in-describe': 'disallow setup calls in describe blocks',
  'no-sibling-hooks': 'disallow duplicate sibling hooks',
  'no-synchronous-tests': 'disallow synchronous tests',
  'no-top-level-hooks': 'disallow top-level hooks',
  'prefer-arrow-callback': 'prefer arrow callbacks in Mocha tests',
  'valid-suite-title': 'require suite titles to match a configured pattern',
  'valid-test-title': 'require test titles to match a configured pattern',
});

const ruleTypes = Object.freeze({
  'consistent-interface': 'problem',
  'consistent-spacing-between-blocks': 'suggestion',
  'handle-done-callback': 'problem',
  'max-top-level-suites': 'suggestion',
  'no-async-suite': 'problem',
  'no-empty-title': 'suggestion',
  'no-exclusive-tests': 'problem',
  'no-exports': 'problem',
  'no-global-tests': 'problem',
  'no-hooks': 'suggestion',
  'no-hooks-for-single-case': 'suggestion',
  'no-identical-title': 'problem',
  'no-mocha-arrows': 'problem',
  'no-nested-tests': 'problem',
  'no-pending-tests': 'problem',
  'no-return-and-callback': 'problem',
  'no-return-from-async': 'problem',
  'no-setup-in-describe': 'problem',
  'no-sibling-hooks': 'problem',
  'no-synchronous-tests': 'suggestion',
  'no-top-level-hooks': 'problem',
  'prefer-arrow-callback': 'suggestion',
  'valid-suite-title': 'suggestion',
  'valid-test-title': 'suggestion',
});

const implementedRuleNames = Object.freeze(implementedMochaRuleNames());
const rules = Object.freeze(
  Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, createMochaRule(ruleName)])),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 0])),
  configs: {
    all: configFromRuleConfig('all', allRuleConfig),
    recommended: configFromRuleConfig('recommended', recommendedRuleConfig),
  },
});

plugin.implementedMochaRuleNames = implementedRuleNames;
plugin.scanMocha = scanMocha;

function configFromRuleConfig(name, ruleConfig) {
  return {
    name: `${PLUGIN_NAME}/${name}`,
    plugins: [PLUGIN_NAME],
    languageOptions: {
      globals: commonGlobals,
    },
    rules: Object.fromEntries(
      Object.entries(ruleConfig).map(([ruleName, config]) => [
        `${PLUGIN_NAME}/${ruleName}`,
        config,
      ]),
    ),
  };
}

function createMochaRule(ruleName) {
  const meta = {
    type: ruleTypes[ruleName],
    docs: {
      description: ruleDescriptions[ruleName],
      category: 'Possible Errors',
      recommended: recommendedRuleConfig[ruleName] !== 'off',
      url: `${DOCS_BASE}#${ruleName}`,
    },
    messages: {
      unexpected: '{{message}}',
    },
    schema: schemaForRule(ruleName),
  };

  return {
    meta,
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

function schemaForRule(ruleName) {
  if (ruleName === 'consistent-interface') {
    return [
      {
        type: 'object',
        properties: {
          interface: { type: 'string', enum: ['BDD', 'TDD'], default: 'BDD' },
        },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'max-top-level-suites') {
    return [
      {
        type: 'object',
        properties: { limit: { type: 'integer' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'handle-done-callback') {
    return [
      {
        type: 'object',
        properties: { ignorePending: { type: 'boolean', default: false } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'no-hooks' || ruleName === 'no-hooks-for-single-case') {
    return [
      {
        type: 'object',
        properties: { allow: { type: 'array', items: { type: 'string' } } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'no-synchronous-tests') {
    return [
      {
        type: 'object',
        properties: {
          allowed: {
            type: 'array',
            items: { type: 'string', enum: ['async', 'callback', 'promise'] },
            minItems: 1,
            uniqueItems: true,
          },
        },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'no-empty-title') {
    return [
      {
        type: 'object',
        properties: { message: { type: 'string' } },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'valid-suite-title' || ruleName === 'valid-test-title') {
    return [
      {
        type: 'object',
        properties: {
          pattern: { type: 'string' },
          message: { type: 'string' },
        },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'prefer-arrow-callback') {
    return [
      {
        type: 'object',
        properties: {
          allowNamedFunctions: { type: 'boolean', default: false },
          allowUnboundThis: { type: 'boolean', default: true },
        },
        additionalProperties: false,
      },
    ];
  }
  return [];
}

function diagnosticsForRule(context, ruleName) {
  return diagnosticsForContext(context, scanOptionsForRule(context, ruleName)).filter(
    (diagnostic) => diagnostic.ruleName === ruleName,
  );
}

function diagnosticsForContext(context, options) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.js';
  const key = JSON.stringify(options);
  let sourceCache = diagnosticsCache.get(sourceCode);

  if (!sourceCache) {
    sourceCache = new Map();
    diagnosticsCache.set(sourceCode, sourceCache);
  }

  const cached = sourceCache.get(key);
  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const diagnostics = scanMocha(sourceText, filename, options);
  sourceCache.set(key, { sourceText, filename, diagnostics });
  return diagnostics;
}

function scanOptionsForRule(context, ruleName) {
  const options =
    context.options?.[0] && typeof context.options[0] === 'object' ? context.options[0] : {};
  const scanOptions = {
    consistentInterface:
      ruleName === 'consistent-interface' ? normalizeInterface(options.interface) : 'BDD',
    maxTopLevelSuitesLimit:
      ruleName === 'max-top-level-suites' && Number.isInteger(options.limit) ? options.limit : 1,
    handleDoneIgnorePending: ruleName === 'handle-done-callback' && options.ignorePending === true,
    noHooksAllowed: ruleName === 'no-hooks' ? normalizeHookNames(options.allow) : [],
    noHooksForSingleCaseAllowed:
      ruleName === 'no-hooks-for-single-case' ? normalizeHookNames(options.allow) : [],
    noEmptyTitleMessage:
      ruleName === 'no-empty-title' && typeof options.message === 'string'
        ? options.message
        : undefined,
    validSuiteTitlePattern:
      ruleName === 'valid-suite-title' && typeof options.pattern === 'string'
        ? options.pattern
        : undefined,
    validSuiteTitleMessage:
      ruleName === 'valid-suite-title' && typeof options.message === 'string'
        ? options.message
        : undefined,
    validTestTitlePattern:
      ruleName === 'valid-test-title'
        ? typeof options.pattern === 'string'
          ? options.pattern
          : '^should'
        : undefined,
    validTestTitleMessage:
      ruleName === 'valid-test-title' && typeof options.message === 'string'
        ? options.message
        : undefined,
    preferArrowAllowNamedFunctions:
      ruleName === 'prefer-arrow-callback' && options.allowNamedFunctions === true,
    preferArrowAllowUnboundThis:
      ruleName !== 'prefer-arrow-callback' || options.allowUnboundThis !== false,
  };

  if (ruleName === 'no-synchronous-tests' && Array.isArray(options.allowed)) {
    scanOptions.noSynchronousAllowed = options.allowed.filter((method) =>
      ['async', 'callback', 'promise'].includes(method),
    );
  }

  return scanOptions;
}

function normalizeInterface(value) {
  return value === 'TDD' ? 'TDD' : 'BDD';
}

function normalizeHookNames(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values
    .filter((value) => typeof value === 'string' && value.length > 0)
    .map((value) => (value.endsWith('()') ? value.slice(0, -2) : value));
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

function reportDiagnostic(context, diagnostic) {
  context.report({
    messageId: 'unexpected',
    data: {
      message: diagnostic.message,
    },
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

module.exports = plugin;
module.exports.default = plugin;
