'use strict';

// Oxlint plugin port of eslint-plugin-perfectionist (MIT).
// The JavaScript layer is only an Oxlint/NAPI adapter; representative sorting
// checks run in Rust through Oxc.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const {
  implementedPerfectionistRuleNames,
  scanPerfectionist,
  scanPerfectionistRule,
} = require('./api.js');

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

function recommendedRules(options) {
  return Object.fromEntries(
    recommendedRuleNames.map((ruleName) => [
      `${PLUGIN_NAME}/${ruleName}`,
      ruleName === 'sort-named-imports' ? ['error', options] : 'error',
    ]),
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
      messages:
        ruleName === 'sort-named-imports'
          ? {
              unexpectedNamedImportsOrder: 'Expected "{{right}}" to come before "{{left}}".',
            }
          : {
              unexpected: 'Expected sorted order.',
            },
      schema: schemaForRule(ruleName),
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
  if (ruleName === 'sort-named-imports') {
    return configuredDiagnosticsForRule(context, ruleName);
  }
  return diagnosticsForContext(context).filter((diagnostic) => diagnostic.ruleName === ruleName);
}

function configuredDiagnosticsForRule(context, ruleName) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.tsx';
  const options = Array.isArray(context.options) ? context.options : [];
  const key = JSON.stringify({ filename, options, ruleName, sourceText });
  let cache = diagnosticsCache.get(sourceCode);

  if (!cache || !(cache instanceof Map)) {
    cache = new Map();
    diagnosticsCache.set(sourceCode, cache);
  }
  if (cache.has(key)) {
    return cache.get(key);
  }
  const diagnostics = scanPerfectionistRule(sourceText, filename, ruleName, options);
  cache.set(key, diagnostics);
  return diagnostics;
}

function diagnosticsForContext(context) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.tsx';
  let cached = diagnosticsCache.get(sourceCode);

  if (
    cached &&
    !(cached instanceof Map) &&
    cached.sourceText === sourceText &&
    cached.filename === filename
  ) {
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
    fix: diagnostic.fix
      ? (fixer) =>
          fixer.replaceTextRange(
            [diagnostic.fix.start, diagnostic.fix.end],
            diagnostic.fix.replacement,
          )
      : undefined,
  });
}

function schemaForRule(ruleName) {
  if (ruleName !== 'sort-named-imports') {
    return [];
  }
  const sortType = {
    type: 'string',
    enum: ['alphabetical', 'natural', 'line-length', 'custom', 'unsorted'],
  };
  const order = {
    type: 'string',
    enum: ['asc', 'desc'],
  };
  return [
    {
      type: 'object',
      properties: {
        type: sortType,
        order,
        ignoreCase: { type: 'boolean' },
        specialCharacters: {
          type: 'string',
          enum: ['keep', 'trim', 'remove'],
        },
        locales: {
          oneOf: [
            { type: 'string' },
            {
              type: 'array',
              items: { type: 'string' },
            },
          ],
        },
        alphabet: { type: 'string' },
        fallbackSort: {
          type: 'object',
          properties: {
            type: sortType,
            order,
          },
          required: ['type'],
          additionalProperties: false,
        },
        ignoreAlias: { type: 'boolean' },
      },
      additionalProperties: false,
    },
  ];
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
module.exports.scanPerfectionistRule = scanPerfectionistRule;
