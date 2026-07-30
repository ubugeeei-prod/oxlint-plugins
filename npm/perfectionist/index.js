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
const configuredArrayRuleNames = new Set(['sort-array-includes', 'sort-sets']);
const configuredRuleNames = new Set([
  'sort-array-includes',
  'sort-exports',
  'sort-imports',
  'sort-named-exports',
  'sort-named-imports',
  'sort-sets',
]);
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
      configuredRuleNames.has(ruleName) ? ['error', options] : 'error',
    ]),
  );
}

function createPerfectionistRule(ruleName) {
  return {
    meta: {
      type: configuredArrayRuleNames.has(ruleName) ? 'suggestion' : 'layout',
      docs: {
        description: `enforce sorted ${ruleName.replace(/^sort-/, '').replaceAll('-', ' ')}`,
        category: 'Stylistic Issues',
        recommended: recommendedRuleNames.includes(ruleName),
        url: `${DOCS_BASE}/${ruleName}`,
      },
      fixable: 'code',
      messages: configuredRuleNames.has(ruleName)
        ? ruleName === 'sort-array-includes'
          ? {
              unexpectedArrayIncludesOrder: 'Expected "{{right}}" to come before "{{left}}".',
              unexpectedArrayIncludesGroupOrder:
                'Expected "{{right}}" ({{rightGroup}}) to come before "{{left}}" ({{leftGroup}}).',
              extraSpacingBetweenArrayIncludesMembers:
                'Extra spacing between "{{left}}" and "{{right}}".',
              missedSpacingBetweenArrayIncludesMembers:
                'Missed spacing between "{{left}}" and "{{right}}".',
            }
          : ruleName === 'sort-sets'
            ? {
                unexpectedSetsOrder: 'Expected "{{right}}" to come before "{{left}}".',
                unexpectedSetsGroupOrder:
                  'Expected "{{right}}" ({{rightGroup}}) to come before "{{left}}" ({{leftGroup}}).',
                extraSpacingBetweenSetsMembers: 'Extra spacing between "{{left}}" and "{{right}}".',
                missedSpacingBetweenSetsMembers:
                  'Missed spacing between "{{left}}" and "{{right}}".',
              }
            : ruleName === 'sort-imports'
              ? {
                  unexpectedImportsOrder: 'Expected "{{right}}" to come before "{{left}}".',
                  unexpectedImportsGroupOrder:
                    'Expected "{{right}}" ({{rightGroup}}) to come before "{{left}}" ({{leftGroup}}).',
                  unexpectedImportsDependencyOrder:
                    'Expected dependency "{{right}}" to come before "{{nodeDependentOnRight}}".',
                  extraSpacingBetweenImports: 'Extra spacing between "{{left}}" and "{{right}}".',
                  missedSpacingBetweenImports: 'Missed spacing between "{{left}}" and "{{right}}".',
                  missedCommentAboveImport:
                    'Missed comment "{{missedCommentAbove}}" above "{{right}}".',
                }
              : ruleName === 'sort-named-imports'
                ? {
                    unexpectedNamedImportsOrder: 'Expected "{{right}}" to come before "{{left}}".',
                    unexpectedNamedImportsGroupOrder:
                      'Expected "{{right}}" ({{rightGroup}}) to come before "{{left}}" ({{leftGroup}}).',
                    extraSpacingBetweenNamedImports:
                      'Extra spacing between "{{left}}" and "{{right}}".',
                    missedSpacingBetweenNamedImports:
                      'Missed spacing between "{{left}}" and "{{right}}".',
                  }
                : ruleName === 'sort-named-exports'
                  ? {
                      unexpectedNamedExportsOrder:
                        'Expected "{{right}}" to come before "{{left}}".',
                      unexpectedNamedExportsGroupOrder:
                        'Expected "{{right}}" ({{rightGroup}}) to come before "{{left}}" ({{leftGroup}}).',
                      extraSpacingBetweenNamedExports:
                        'Extra spacing between "{{left}}" and "{{right}}".',
                      missedSpacingBetweenNamedExports:
                        'Missed spacing between "{{left}}" and "{{right}}".',
                    }
                  : {
                      unexpectedExportsOrder: 'Expected "{{right}}" to come before "{{left}}".',
                      unexpectedExportsGroupOrder:
                        'Expected "{{right}}" ({{rightGroup}}) to come before "{{left}}" ({{leftGroup}}).',
                      extraSpacingBetweenExports:
                        'Extra spacing between "{{left}}" and "{{right}}".',
                      missedSpacingBetweenExports:
                        'Missed spacing between "{{left}}" and "{{right}}".',
                      missedCommentAboveExport:
                        'Missed comment "{{missedCommentAbove}}" above "{{right}}".',
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
  if (configuredRuleNames.has(ruleName)) {
    return configuredDiagnosticsForRule(context, ruleName);
  }
  return diagnosticsForContext(context).filter((diagnostic) => diagnostic.ruleName === ruleName);
}

function configuredDiagnosticsForRule(context, ruleName) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.tsx';
  let options = Array.isArray(context.options) ? context.options : [];
  if (
    configuredArrayRuleNames.has(ruleName) ||
    ruleName === 'sort-exports' ||
    ruleName === 'sort-imports'
  ) {
    const settings = context.settings?.perfectionist;
    if (settings && typeof settings === 'object' && !Array.isArray(settings)) {
      options =
        options.length === 0
          ? [{ ...settings }]
          : options.map((configured) => ({
              ...settings,
              ...(configured && typeof configured === 'object' && !Array.isArray(configured)
                ? configured
                : {}),
            }));
    }
  }
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
  const data = Object.fromEntries(
    Object.entries(diagnostic.data || {}).filter(([, value]) => value !== undefined),
  );
  context.report({
    messageId: diagnostic.messageId,
    data,
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
  if (!configuredRuleNames.has(ruleName)) {
    return [];
  }
  const sortsArrayRule = configuredArrayRuleNames.has(ruleName);
  const selector =
    ruleName === 'sort-named-imports' ? 'import' : sortsArrayRule ? 'literal' : 'export';
  const sortsExportDeclarations = ruleName === 'sort-exports';
  const sortsImportDeclarations = ruleName === 'sort-imports';
  const sortType = {
    type: 'string',
    enum: [
      'subgroup-order',
      'alphabetical',
      'natural',
      'line-length',
      'custom',
      'unsorted',
      ...(sortsImportDeclarations ? ['type-import-first'] : []),
    ],
  };
  const order = {
    type: 'string',
    enum: ['asc', 'desc'],
  };
  const singleRegex = {
    oneOf: [
      { type: 'string' },
      {
        type: 'object',
        properties: {
          pattern: { type: 'string' },
          flags: { type: 'string' },
        },
        required: ['pattern'],
        additionalProperties: false,
      },
    ],
  };
  const regex = {
    oneOf: [
      ...singleRegex.oneOf,
      {
        type: 'array',
        items: singleRegex,
      },
    ],
  };
  const newlines = {
    oneOf: [
      { type: 'number', minimum: 0 },
      { type: 'string', enum: ['ignore'] },
    ],
  };
  const newlinesInside = {
    oneOf: [
      { type: 'number', minimum: 0 },
      { type: 'string', enum: ['ignore', 'newlinesBetween'] },
    ],
  };
  const fallbackSort = {
    type: 'object',
    properties: {
      type: sortType,
      order,
    },
    required: ['type'],
    additionalProperties: false,
  };
  const sortOverrides = {
    type: sortType,
    order,
    fallbackSort,
    ...(sortsImportDeclarations ? { sortBy: { type: 'string', enum: ['specifier', 'path'] } } : {}),
  };
  const customMatch = {
    elementNamePattern: regex,
    ...(sortsArrayRule
      ? {}
      : {
          modifiers: {
            type: 'array',
            items: {
              type: 'string',
              enum: sortsImportDeclarations
                ? [
                    'default',
                    'multiline',
                    'named',
                    'require',
                    'side-effect',
                    'singleline',
                    'ts-equals',
                    'type',
                    'value',
                    'wildcard',
                  ]
                : sortsExportDeclarations
                  ? ['value', 'type', 'named', 'wildcard', 'singleline', 'multiline']
                  : ['value', 'type'],
            },
          },
        }),
    selector: {
      type: 'string',
      enum: sortsImportDeclarations
        ? [
            'side-effect-style',
            'tsconfig-path',
            'side-effect',
            'external',
            'internal',
            'builtin',
            'sibling',
            'subpath',
            'import',
            'parent',
            'index',
            'style',
            'type',
          ]
        : [selector],
    },
  };
  const groups = {
    type: 'array',
    items: {
      oneOf: [
        { type: 'string' },
        {
          type: 'array',
          minItems: 1,
          items: { type: 'string' },
        },
        {
          type: 'object',
          properties: { newlinesBetween: newlines },
          required: ['newlinesBetween'],
          additionalProperties: false,
        },
        {
          type: 'object',
          properties: {
            group: {
              oneOf: [
                { type: 'string' },
                {
                  type: 'array',
                  minItems: 1,
                  items: { type: 'string' },
                },
              ],
            },
            ...sortOverrides,
            newlinesInside,
            commentAbove: { type: 'string' },
          },
          required: ['group'],
          minProperties: 2,
          additionalProperties: false,
        },
      ],
    },
  };
  const customGroups = {
    type: 'array',
    items: {
      oneOf: [
        {
          type: 'object',
          properties: {
            groupName: { type: 'string' },
            ...sortOverrides,
            newlinesInside: newlines,
            ...customMatch,
          },
          required: ['groupName'],
          minProperties: 2,
          additionalProperties: false,
        },
        {
          type: 'object',
          properties: {
            groupName: { type: 'string' },
            ...sortOverrides,
            newlinesInside: newlines,
            anyOf: {
              type: 'array',
              minItems: 1,
              items: {
                type: 'object',
                properties: customMatch,
                additionalProperties: false,
              },
            },
          },
          required: ['groupName', 'anyOf'],
          additionalProperties: false,
        },
      ],
    },
  };
  const partitionByComment = {
    oneOf: [
      { type: 'boolean' },
      ...regex.oneOf,
      {
        type: 'object',
        properties: {
          block: { oneOf: [{ type: 'boolean' }, ...regex.oneOf] },
          line: { oneOf: [{ type: 'boolean' }, ...regex.oneOf] },
        },
        minProperties: 1,
        additionalProperties: false,
      },
    ],
  };
  const optionSchema = {
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
      fallbackSort,
      ...(!sortsArrayRule && !sortsExportDeclarations && !sortsImportDeclarations
        ? { ignoreAlias: { type: 'boolean' } }
        : {}),
      groups,
      customGroups,
      partitionByComment,
      partitionByNewLine: { type: 'boolean' },
      newlinesBetween: newlines,
      newlinesInside,
      ...(sortsImportDeclarations
        ? {
            sortBy: { type: 'string', enum: ['specifier', 'path'] },
            internalPattern: regex,
            environment: { type: 'string', enum: ['node', 'bun'] },
            sortSideEffects: { type: 'boolean' },
            maxLineLength: { type: 'integer', minimum: 0, exclusiveMinimum: true },
            useExperimentalDependencyDetection: { type: 'boolean' },
            tsconfig: {
              type: 'object',
              properties: {
                rootDir: { type: 'string' },
                filename: { type: 'string' },
              },
              required: ['rootDir'],
              additionalProperties: false,
            },
          }
        : sortsExportDeclarations
          ? {}
          : {
              useConfigurationIf: {
                type: 'object',
                properties: {
                  allNamesMatchPattern: regex,
                  matchesAstSelector: { type: 'string' },
                },
                ...(sortsArrayRule ? {} : { minProperties: 1 }),
                additionalProperties: false,
              },
            }),
    },
    additionalProperties: false,
  };
  return sortsArrayRule
    ? {
        items: optionSchema,
        uniqueItems: true,
        type: 'array',
      }
    : [optionSchema];
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
