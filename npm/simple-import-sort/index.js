'use strict';

// Oxlint plugin port of eslint-plugin-simple-import-sort (MIT).
// Parsing, import/export chunk discovery, sort decisions, and fix ranges run in
// Rust through Oxc. The JS layer only adapts Oxlint plugin APIs.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedSimpleImportSortRuleNames, scanSimpleImportSort } = require('./api.js');

const PLUGIN_NAME = 'simple-import-sort';
const DOCS_BASE =
  'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/simple-import-sort';
const diagnosticsCache = new WeakMap();

const messages = Object.freeze({
  imports: {
    sort: 'Run autofix to sort these imports!',
  },
  exports: {
    sort: 'Run autofix to sort these exports!',
  },
});

const ruleDescriptions = Object.freeze({
  imports: 'automatically sort imports',
  exports: 'automatically sort exports',
});

const implementedRuleNames = Object.freeze(implementedSimpleImportSortRuleNames());
const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createSimpleImportSortRule(ruleName)]),
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
    recommended: {
      name: `${PLUGIN_NAME}/recommended`,
      plugins: [PLUGIN_NAME],
      rules: {
        [`${PLUGIN_NAME}/imports`]: 'error',
        [`${PLUGIN_NAME}/exports`]: 'error',
      },
    },
  },
});

plugin.implementedSimpleImportSortRuleNames = implementedRuleNames;
plugin.scanSimpleImportSort = scanSimpleImportSort;

function createSimpleImportSortRule(ruleName) {
  return {
    meta: {
      type: 'layout',
      docs: {
        description: ruleDescriptions[ruleName],
        category: 'Stylistic Issues',
        recommended: true,
        url: `${DOCS_BASE}#${ruleName}`,
      },
      fixable: 'code',
      messages: messages[ruleName],
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

function schemaForRule(ruleName) {
  if (ruleName !== 'imports') {
    return [];
  }
  return [
    {
      type: 'object',
      properties: {
        groups: {
          type: 'array',
          items: {
            type: 'array',
            items: { type: 'string' },
          },
        },
      },
      additionalProperties: false,
    },
  ];
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

  const diagnostics = scanSimpleImportSort(sourceText, filename, options);
  sourceCache.set(key, { sourceText, filename, diagnostics });
  return diagnostics;
}

function scanOptionsForRule(context, ruleName) {
  const options =
    context.options?.[0] && typeof context.options[0] === 'object' ? context.options[0] : {};
  return {
    importGroups:
      ruleName === 'imports' && Array.isArray(options.groups) ? options.groups : undefined,
  };
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
    fix: diagnostic.fix
      ? (fixer) =>
          fixer.replaceTextRange(
            [diagnostic.fix.start, diagnostic.fix.end],
            diagnostic.fix.replacement,
          )
      : undefined,
  });
}

module.exports = plugin;
module.exports.default = plugin;
