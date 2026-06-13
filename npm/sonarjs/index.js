'use strict';

// Oxlint plugin port of eslint-plugin-sonarjs (upstream is LGPL-3.0).
// Clean-room implementation: behaviour is reproduced from public RSPEC docs and
// observed output only. The JavaScript layer adapts Oxlint's ESLint-compatible
// plugin API; parsing and rule checks run in Rust through Oxc. Message strings
// live here (independently authored), not in the Rust core.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedSonarjsRuleNames, scanSonarjs } = require('./api.js');

const PLUGIN_NAME = 'sonarjs';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/sonarjs';
const diagnosticsCache = new WeakMap();

const messages = Object.freeze({
  'no-nested-template-literals': {
    nestedTemplateLiteral:
      'Do not nest template literals. Extract the inner template literal into a separate variable.',
  },
  'no-nested-switch': {
    nestedSwitch:
      'Do not nest switch statements. Extract the nested switch into a separate function.',
  },
  'no-nested-conditional': {
    nestedConditional:
      'Do not nest ternary/conditional expressions; extract the nested conditional into an independent statement.',
  },
  'no-collapsible-if': {
    collapsibleIf: "Merge this 'if' statement with the nested one to reduce nesting.",
  },
  'no-redundant-boolean': {
    redundantBoolean: 'Remove this redundant boolean literal.',
  },
  'comma-or-logical-or-case': {
    commaOrLogicalOrInCase:
      "This 'case' label uses '||' or ',', which does not compare against multiple values as it appears to.",
  },
  'no-duplicate-in-composite': {
    duplicateType: 'Remove this duplicated type or replace with another one.',
  },
  'non-existent-operator': {
    nonExistentOperator:
      "Was this '=-', '=+', or '=!' meant to be a compound assignment or comparison? Add a space to clarify, or fix the operator.",
  },
  'no-identical-conditions': {
    identicalConditions:
      "This branch's condition duplicates an earlier one in the same if/else-if chain, so it can never be reached.",
  },
  'no-all-duplicated-branches': {
    allDuplicatedBranches:
      'Remove this conditional structure or edit its code blocks so that they are not all the same.',
  },
});

const ruleDescriptions = Object.freeze({
  'no-nested-template-literals': 'Disallow nested template literals',
  'no-nested-switch': 'Disallow nested switch statements',
  'no-nested-conditional': 'Disallow nested conditional (ternary) expressions',
  'no-collapsible-if': 'Disallow collapsible if statements that should be merged',
  'no-redundant-boolean': 'Disallow redundant boolean literals in expressions',
  'comma-or-logical-or-case': "Disallow '||' or ',' expressions as switch case labels",
  'no-duplicate-in-composite':
    'Disallow duplicate type members in TypeScript union or intersection types',
  'non-existent-operator':
    "Disallow the suspicious '=-', '=+', or '=!' operator typos adjacent to a plain assignment",
  'no-identical-conditions':
    'Disallow duplicate conditions in the same if/else-if chain (dead branch)',
  'no-all-duplicated-branches':
    'Disallow conditional structures where every branch has the same implementation',
});

const ruleTypes = Object.freeze({
  'no-nested-template-literals': 'suggestion',
  'no-nested-switch': 'suggestion',
  'no-nested-conditional': 'suggestion',
  'no-collapsible-if': 'suggestion',
  'no-redundant-boolean': 'suggestion',
  'comma-or-logical-or-case': 'suggestion',
  'no-duplicate-in-composite': 'suggestion',
  'non-existent-operator': 'problem',
  'no-identical-conditions': 'problem',
  'no-all-duplicated-branches': 'problem',
});

const recommendedRuleConfig = Object.freeze({
  'no-nested-template-literals': 'error',
  'no-nested-switch': 'error',
  'no-nested-conditional': 'error',
  'no-collapsible-if': 'error',
  'no-redundant-boolean': 'error',
  'comma-or-logical-or-case': 'error',
  'no-duplicate-in-composite': 'error',
  'non-existent-operator': 'error',
  'no-identical-conditions': 'error',
  'no-all-duplicated-branches': 'error',
});

const implementedRuleNames = Object.freeze(implementedSonarjsRuleNames());
const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createSonarjsRule(ruleName)]),
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

plugin.implementedSonarjsRuleNames = implementedRuleNames;
plugin.scanSonarjs = scanSonarjs;

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

function createSonarjsRule(ruleName) {
  return {
    meta: {
      type: ruleTypes[ruleName],
      docs: {
        description: ruleDescriptions[ruleName],
        recommended: recommendedRuleConfig[ruleName] !== undefined,
        url: `${DOCS_BASE}#${ruleName}`,
      },
      messages: messages[ruleName],
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
  return diagnosticsForContext(context, { ruleNames: [ruleName] }).filter(
    (diagnostic) => diagnostic.ruleName === ruleName,
  );
}

function diagnosticsForContext(context, options) {
  const sourceCode = context.sourceCode ?? context.getSourceCode?.() ?? {};
  const sourceText = sourceTextForContext(context);
  const filename = context.filename ?? context.getFilename?.() ?? 'file.js';
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

  const diagnostics = scanSonarjs(sourceText, filename, options);
  sourceCache.set(key, { sourceText, filename, diagnostics });
  return diagnostics;
}

function sourceTextForContext(context) {
  const sourceCode = context.sourceCode ?? context.getSourceCode?.() ?? {};
  if (typeof sourceCode.getText === 'function') {
    return sourceCode.getText();
  }
  if (typeof sourceCode.text === 'string') {
    return sourceCode.text;
  }
  return '';
}

function reportDiagnostic(context, diagnostic) {
  const report = {
    messageId: diagnostic.messageId,
    data: compactData(diagnostic.data),
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

function compactData(data) {
  const out = {};
  for (const [key, value] of Object.entries(data || {})) {
    if (value != null) {
      out[key] = value;
    }
  }
  return out;
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedSonarjsRuleNames = implementedRuleNames;
module.exports.scanSonarjs = scanSonarjs;
