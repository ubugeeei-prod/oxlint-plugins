'use strict';

// Oxlint plugin port of @angular-eslint/eslint-plugin (MIT).
// The JavaScript layer is an Oxlint/NAPI adapter; Angular-focused scans run in
// Rust through Oxc-backed source parsing and fast structural pattern checks.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedAngularEslintRuleNames, scanAngularEslint } = require('./api.js');

const PLUGIN_NAME = '@angular-eslint';
const DOCS_BASE =
  'https://github.com/angular-eslint/angular-eslint/blob/main/packages/eslint-plugin/docs/rules';
const diagnosticsCache = new WeakMap();
const implementedRuleNames = Object.freeze(implementedAngularEslintRuleNames());

const problemRules = new Set([
  'computed-must-return',
  'contextual-lifecycle',
  'no-async-lifecycle-method',
  'no-attribute-decorator',
  'no-developer-preview',
  'no-empty-lifecycle-method',
  'no-experimental',
  'no-lifecycle-call',
  'require-lifecycle-on-prototype',
]);

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createAngularEslintRule(ruleName)]),
  ),
);

const allRuleConfig = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
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
    all: {
      name: `${PLUGIN_NAME}/all`,
      plugins: [PLUGIN_NAME],
      rules: allRuleConfig,
    },
  },
});

plugin.implementedAngularEslintRuleNames = implementedRuleNames;
plugin.scanAngularEslint = scanAngularEslint;

function createAngularEslintRule(ruleName) {
  return {
    meta: {
      type: problemRules.has(ruleName) ? 'problem' : 'suggestion',
      docs: {
        description: `enforce angular eslint ${ruleName.replaceAll('-', ' ')}`,
        category: 'Best Practices',
        recommended: false,
        url: `${DOCS_BASE}/${ruleName}.md`,
      },
      messages: {
        unexpected: 'Unexpected Angular pattern.',
      },
      // These rules do not yet honor upstream options (the core is a heuristic
      // scanner). Declare no schema so configured options are surfaced as an
      // error rather than silently ignored. Tracked for real implementation.
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
  const filename = typeof context.filename === 'string' ? context.filename : 'file.ts';
  let cached = diagnosticsCache.get(sourceCode);

  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const diagnostics = scanAngularEslint(sourceText, filename);
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
module.exports.implementedAngularEslintRuleNames = implementedRuleNames;
module.exports.scanAngularEslint = scanAngularEslint;
