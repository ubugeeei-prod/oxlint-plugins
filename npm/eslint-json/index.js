'use strict';

// Oxlint plugin port of @eslint/json (MIT).
// The JavaScript layer is only an Oxlint/NAPI adapter; JSON, JSONC, and JSON5
// tokenization plus rule checks run in Rust.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedEslintJsonRuleNames, scanEslintJson } = require('./api.js');

const PLUGIN_NAME = 'json';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/eslint-json';
const diagnosticsCache = new WeakMap();

const messages = {
  'no-duplicate-keys': {
    duplicateKey: 'Duplicate key "{{key}}" found.',
  },
  'no-empty-keys': {
    emptyKey: 'Empty key found.',
  },
  'no-unnormalized-keys': {
    unnormalizedKey: "Unnormalized key '{{key}}' found.",
  },
  'no-unsafe-values': {
    unsafeNumber: "The number '{{ value }}' will evaluate to Infinity.",
    unsafeInteger: "The integer '{{ value }}' is outside the safe integer range.",
    unsafeZero: "The number '{{ value }}' will evaluate to zero.",
    subnormal:
      "Unexpected subnormal number '{{ value }}' found, which may cause interoperability issues.",
    loneSurrogate: "Lone surrogate '{{ surrogate }}' found.",
  },
  'sort-keys': {
    sortKeys:
      "Expected object keys to be in {{sortName}} case-{{sensitivity}} {{direction}} order. '{{thisName}}' should be before '{{prevName}}'.",
  },
  'top-level-interop': {
    topLevel: "Top level item should be array or object, got '{{type}}'.",
  },
};

const ruleDescriptions = {
  'no-duplicate-keys': 'disallow duplicate keys in JSON objects',
  'no-empty-keys': 'disallow empty keys in JSON objects',
  'no-unnormalized-keys': 'disallow JSON keys that are not normalized',
  'no-unsafe-values': 'disallow JSON values that are unsafe for interchange',
  'sort-keys': 'require JSON object keys to be sorted',
  'top-level-interop': 'require the JSON top-level value to be an array or object',
};

const recommendedRuleNames = Object.freeze([
  'no-duplicate-keys',
  'no-empty-keys',
  'no-unnormalized-keys',
  'no-unsafe-values',
]);

const implementedRuleNames = Object.freeze(implementedEslintJsonRuleNames());

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createEslintJsonRule(ruleName)]),
  ),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: '@eslint/json',
    namespace: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 0])),
  configs: {
    recommended: {
      name: '@eslint/json/recommended',
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        recommendedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
      ),
    },
    all: {
      name: '@eslint/json/all',
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
      ),
    },
  },
});

plugin.implementedEslintJsonRuleNames = implementedRuleNames;
plugin.scanEslintJson = scanEslintJson;

function createEslintJsonRule(ruleName) {
  const meta = {
    type: ruleName === 'sort-keys' ? 'suggestion' : 'problem',
    docs: {
      description: ruleDescriptions[ruleName],
      recommended: recommendedRuleNames.includes(ruleName),
      url: `${DOCS_BASE}#${ruleName}`,
    },
    languages: ['json/json', 'json/jsonc', 'json/json5'],
    messages: messages[ruleName],
    schema: schemaForRule(ruleName),
  };

  if (ruleName === 'no-unnormalized-keys') {
    meta.fixable = 'code';
    meta.defaultOptions = [{ form: 'NFC' }];
  }
  if (ruleName === 'sort-keys') {
    meta.fixable = 'code';
    meta.defaultOptions = [
      'asc',
      {
        allowLineSeparatedGroups: false,
        caseSensitive: true,
        minKeys: 2,
        natural: false,
      },
    ];
  }

  return {
    meta,
    createOnce(context) {
      return {
        Program() {
          for (const diagnostic of diagnosticsForContext(ruleName, context)) {
            context.report(reportDescriptor(diagnostic));
          }
        },
      };
    },
  };
}

function diagnosticsForContext(ruleName, context) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const options = scanOptionsForRule(ruleName, context.options || []);
  const cacheKey = JSON.stringify(options);
  const cached = diagnosticsCache.get(sourceCode);

  if (cached && cached.sourceText === sourceText && cached.cacheKey === cacheKey) {
    return cached.diagnostics;
  }

  const diagnostics = scanEslintJson(sourceText, options);
  diagnosticsCache.set(sourceCode, { sourceText, cacheKey, diagnostics });
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

function scanOptionsForRule(ruleName, ruleOptions) {
  if (ruleName === 'no-unnormalized-keys') {
    const [raw = {}] = ruleOptions;
    return {
      ruleNames: [ruleName],
      normalizationForm: normalizeNormalizationForm(raw.form),
    };
  }

  if (ruleName === 'sort-keys') {
    const [direction = 'asc', raw = {}] = ruleOptions;
    return {
      ruleNames: [ruleName],
      sortDirection: direction === 'desc' ? 'desc' : 'asc',
      sortAllowLineSeparatedGroups: raw.allowLineSeparatedGroups === true,
      sortCaseSensitive: raw.caseSensitive !== false,
      sortMinKeys: Number.isInteger(raw.minKeys) && raw.minKeys >= 2 ? raw.minKeys : 2,
      sortNatural: raw.natural === true,
    };
  }

  return {
    ruleNames: [ruleName],
  };
}

function normalizeNormalizationForm(value) {
  return ['NFC', 'NFD', 'NFKC', 'NFKD'].includes(value) ? value : 'NFC';
}

function reportDescriptor(diagnostic) {
  const descriptor = {
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
    descriptor.fix = (fixer) =>
      fixer.replaceTextRange(
        [diagnostic.fix.start, diagnostic.fix.end],
        diagnostic.fix.replacement,
      );
  }

  return descriptor;
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

function schemaForRule(ruleName) {
  if (ruleName === 'no-unnormalized-keys') {
    return [
      {
        type: 'object',
        properties: {
          form: {
            enum: ['NFC', 'NFD', 'NFKC', 'NFKD'],
          },
        },
        additionalProperties: false,
      },
    ];
  }

  if (ruleName === 'sort-keys') {
    return [
      {
        enum: ['asc', 'desc'],
      },
      {
        type: 'object',
        properties: {
          allowLineSeparatedGroups: {
            type: 'boolean',
          },
          caseSensitive: {
            type: 'boolean',
          },
          minKeys: {
            type: 'integer',
            minimum: 2,
          },
          natural: {
            type: 'boolean',
          },
        },
        additionalProperties: false,
      },
    ];
  }

  return [];
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedEslintJsonRuleNames = implementedRuleNames;
module.exports.scanEslintJson = scanEslintJson;
