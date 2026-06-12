'use strict';

// Oxlint plugin port of eslint-plugin-unused-imports (MIT).
// The JavaScript layer adapts Oxlint's ESLint-compatible plugin API to the
// Rust/Oxc implementation. Import usage, scope resolution, JSDoc handling, and
// autofix range calculation run in native code.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedUnusedImportsRuleNames, scanUnusedImports } = require('./api.js');

const PLUGIN_NAME = 'unused-imports';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/unused-imports';
const diagnosticsCache = new WeakMap();

const recommendedRuleConfig = Object.freeze({
  'no-unused-imports': 'error',
  'no-unused-vars': 'off',
});

const ruleDescriptions = Object.freeze({
  'no-unused-imports': 'disallow unused imports',
  'no-unused-vars': 'disallow unused variables while leaving import reporting to no-unused-imports',
});

const implementedRuleNames = Object.freeze(implementedUnusedImportsRuleNames());
const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createUnusedImportsRule(ruleName)]),
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
  },
});

plugin.implementedUnusedImportsRuleNames = implementedRuleNames;
plugin.scanUnusedImports = scanUnusedImports;

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

function createUnusedImportsRule(ruleName) {
  return {
    meta: {
      type: 'problem',
      docs: {
        description: ruleDescriptions[ruleName],
        category: 'Variables',
        recommended: recommendedRuleConfig[ruleName] !== 'off',
        url: `${DOCS_BASE}#${ruleName}`,
      },
      fixable: ruleName === 'no-unused-imports' ? 'code' : undefined,
      messages: {
        unused: '{{message}}',
      },
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
  const sourceCode = context.sourceCode ?? context.getSourceCode();
  let bySource = diagnosticsCache.get(sourceCode);
  if (!bySource) {
    bySource = new Map();
    diagnosticsCache.set(sourceCode, bySource);
  }
  const filename = context.filename ?? context.getFilename?.() ?? 'file.js';
  const cacheKey = `${filename}\0${ruleName}`;
  let diagnostics = bySource.get(cacheKey);
  if (!diagnostics) {
    diagnostics = scanUnusedImports(sourceCode.text, filename, { ruleNames: [ruleName] }).filter(
      (diagnostic) => diagnostic.ruleName === ruleName,
    );
    bySource.set(cacheKey, diagnostics);
  }
  return diagnostics;
}

function reportDiagnostic(context, diagnostic) {
  const report = {
    messageId: 'unused',
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
  };

  if (diagnostic.fix) {
    report.fix = (fixer) =>
      fixer.replaceTextRange(
        [diagnostic.fix.start, diagnostic.fix.end],
        diagnostic.fix.replacement,
      );
  }

  context.report(report);
}

module.exports = plugin;
module.exports.default = plugin;
