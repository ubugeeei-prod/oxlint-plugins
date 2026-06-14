'use strict';

// Oxlint plugin port of eslint-plugin-perfectionist (MIT).
// The JavaScript layer is only an Oxlint/NAPI adapter; representative sorting
// checks run in Rust through Oxc.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedPerfectionistRuleNames, scanPerfectionist } = require('./api.js');

const PLUGIN_NAME = 'perfectionist';
const DOCS_BASE = 'https://perfectionist.dev/rules';
const diagnosticsCache = new WeakMap();
const implementedRuleNames = Object.freeze(implementedPerfectionistRuleNames());
const recommendedRuleNames = Object.freeze(
  implementedRuleNames.filter((ruleName) => ruleName !== 'sort-arrays'),
);

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createPerfectionistRule(ruleName)]),
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
    'recommended-alphabetical': createConfig({ type: 'alphabetical', order: 'asc' }),
    'recommended-natural': createConfig({ type: 'natural', order: 'asc' }),
    'recommended-line-length': createConfig({ type: 'line-length', order: 'desc' }),
    'recommended-custom': createConfig({ type: 'custom', order: 'asc' }),
    'recommended-alphabetical-legacy': createLegacyConfig({
      type: 'alphabetical',
      order: 'asc',
    }),
    'recommended-natural-legacy': createLegacyConfig({ type: 'natural', order: 'asc' }),
    'recommended-line-length-legacy': createLegacyConfig({
      type: 'line-length',
      order: 'desc',
    }),
    'recommended-custom-legacy': createLegacyConfig({ type: 'custom', order: 'asc' }),
  },
});

plugin.implementedPerfectionistRuleNames = implementedRuleNames;
plugin.scanPerfectionist = scanPerfectionist;

function createConfig(options) {
  return {
    name: `${PLUGIN_NAME}/recommended-${options.type}`,
    plugins: [PLUGIN_NAME],
    rules: recommendedRules(options),
  };
}

function createLegacyConfig(options) {
  return {
    plugins: [PLUGIN_NAME],
    rules: recommendedRules(options),
  };
}

function recommendedRules() {
  // The core sorts with fixed defaults and does not honor option values yet, so
  // configs enable rules at `error` without an (ignored) options payload — which
  // would also violate the now-empty per-rule schema. The `recommended-*`
  // variants therefore behave identically until the sort engine lands.
  return Object.fromEntries(
    recommendedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
  );
}

function createPerfectionistRule(ruleName) {
  return {
    meta: {
      type: 'layout',
      docs: {
        description: `enforce sorted ${ruleName.replace(/^sort-/, '').replaceAll('-', ' ')}`,
        category: 'Stylistic Issues',
        recommended: recommendedRuleNames.includes(ruleName),
        url: `${DOCS_BASE}/${ruleName}`,
      },
      fixable: 'code',
      messages: {
        unexpected: 'Expected sorted order.',
      },
      // The core sorts with fixed defaults and does not honor upstream options
      // yet, so declare no schema rather than silently ignore configured
      // options. Tracked for implementation of the configurable sort engine.
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

function diagnosticsForRule(context, ruleName) {
  return diagnosticsForContext(context).filter((diagnostic) => diagnostic.ruleName === ruleName);
}

function diagnosticsForContext(context) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.tsx';
  let cached = diagnosticsCache.get(sourceCode);

  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const diagnostics = scanPerfectionist(sourceText, filename);
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

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedPerfectionistRuleNames = implementedRuleNames;
module.exports.scanPerfectionist = scanPerfectionist;
