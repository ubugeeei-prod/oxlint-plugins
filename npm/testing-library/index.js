'use strict';

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedTestingLibraryRuleNames, scanTestingLibrary } = require('./api.js');

const PLUGIN_NAME = 'testing-library';
const DOCS_BASE =
  'https://github.com/testing-library/eslint-plugin-testing-library/blob/main/docs/rules';
const diagnosticsCache = new WeakMap();

const ruleDescriptions = Object.freeze({
  'await-async-events': 'enforce promises from async event methods are handled',
  'await-async-queries': 'enforce promises from async queries are handled',
  'await-async-utils': 'enforce promises from async utils are handled',
  'consistent-data-testid': 'enforce consistent data-testid values',
  'no-await-sync-events': 'disallow awaiting sync events',
  'no-await-sync-queries': 'disallow awaiting sync queries',
  'no-container': 'disallow container access',
  'no-debugging-utils': 'disallow debugging utilities',
  'no-dom-import': 'disallow DOM Testing Library imports',
  'no-global-regexp-flag-in-query': 'disallow global regex flags in queries',
  'no-manual-cleanup': 'disallow manual cleanup',
  'no-node-access': 'disallow direct node access',
  'no-promise-in-fire-event': 'disallow promises in fireEvent calls',
  'no-render-in-lifecycle': 'disallow render in lifecycle hooks',
  'no-test-id-queries': 'disallow test id queries',
  'no-unnecessary-act': 'disallow unnecessary act wrappers',
  'no-wait-for-multiple-assertions': 'disallow multiple assertions in waitFor',
  'no-wait-for-side-effects': 'disallow side effects in waitFor',
  'no-wait-for-snapshot': 'disallow snapshots in waitFor',
  'prefer-explicit-assert': 'prefer explicit assertions',
  'prefer-find-by': 'prefer findBy queries',
  'prefer-implicit-assert': 'prefer implicit query assertions',
  'prefer-presence-queries': 'prefer presence queries matching assertions',
  'prefer-query-by-disappearance': 'prefer queryBy for disappearance',
  'prefer-query-matchers': 'prefer query matchers',
  'prefer-screen-queries': 'prefer screen queries',
  'prefer-user-event': 'prefer userEvent',
  'prefer-user-event-setup': 'prefer userEvent.setup',
  'render-result-naming-convention': 'enforce render result naming',
});

const recommendedRuleConfig = Object.freeze(
  Object.fromEntries(Object.keys(ruleDescriptions).map((ruleName) => [ruleName, 'error'])),
);

const implementedRuleNames = Object.freeze(implementedTestingLibraryRuleNames());
const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createTestingLibraryRule(ruleName)]),
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
    recommended: configFromRuleConfig('recommended', recommendedRuleConfig),
    all: configFromRuleConfig(
      'all',
      Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 'error'])),
    ),
    off: configFromRuleConfig(
      'off',
      Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 'off'])),
    ),
  },
});

plugin.implementedTestingLibraryRuleNames = implementedRuleNames;
plugin.scanTestingLibrary = scanTestingLibrary;

function configFromRuleConfig(name, ruleConfig) {
  return {
    name: `${PLUGIN_NAME}/${name}`,
    plugins: [PLUGIN_NAME],
    rules: Object.fromEntries(
      Object.entries(ruleConfig).map(([ruleName, config]) => [
        `${PLUGIN_NAME}/${ruleName}`,
        config,
      ]),
    ),
  };
}

function createTestingLibraryRule(ruleName) {
  return {
    meta: {
      type: 'problem',
      docs: {
        description: ruleDescriptions[ruleName],
        recommended: true,
        url: `${DOCS_BASE}/${ruleName}.md`,
      },
      messages: {
        unexpected: '{{message}}',
      },
      schema:
        ruleName === 'consistent-data-testid'
          ? [{ type: 'object', additionalProperties: true }]
          : [],
    },
    createOnce(context) {
      return {
        Program() {
          for (const diagnostic of diagnosticsForRule(context, ruleName)) {
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
        },
      };
    },
  };
}

function diagnosticsForRule(context, ruleName) {
  return diagnosticsForContext(context, { ruleNames: [ruleName] }).filter(
    (diagnostic) => diagnostic.ruleName === ruleName,
  );
}

function diagnosticsForContext(context, options) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.test.tsx';
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

  const diagnostics = scanTestingLibrary(sourceText, filename, options);
  sourceCache.set(key, { sourceText, filename, diagnostics });
  return diagnostics;
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

module.exports = plugin;
module.exports.default = plugin;
