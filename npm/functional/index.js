'use strict';

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedFunctionalRuleNames, scanFunctional } = require('./api.js');

const PLUGIN_NAME = 'functional';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/functional';
const diagnosticsCache = new WeakMap();

const ruleDescriptions = Object.freeze({
  'functional-parameters': 'enforce functional parameters',
  'immutable-data': 'enforce treating data as immutable',
  'no-class-inheritance': 'disallow inheritance in classes',
  'no-classes': 'disallow classes',
  'no-conditional-statements': 'disallow conditional statements',
  'no-expression-statements': 'disallow expression statements',
  'no-let': 'disallow mutable variables',
  'no-loop-statements': 'disallow imperative loops',
  'no-mixed-types': 'restrict type members to the same kind',
  'no-promise-reject': 'disallow rejecting promises',
  'no-return-void': 'disallow functions that do not return values',
  'no-this-expressions': 'disallow this expressions',
  'no-throw-statements': 'disallow throwing exceptions',
  'no-try-statements': 'disallow try/catch/finally statements',
  'prefer-immutable-types': 'prefer immutable TypeScript types',
  'prefer-property-signatures': 'prefer property signatures over method signatures',
  'prefer-readonly-type': 'prefer readonly TypeScript types',
  'prefer-tacit': 'prefer tacit function references',
  'readonly-type': 'enforce readonly type style',
  'type-declaration-immutability': 'enforce immutability of type declarations',
});

const ruleTypes = Object.freeze(
  Object.fromEntries(
    Object.keys(ruleDescriptions).map((ruleName) => [
      ruleName,
      ruleName === 'no-promise-reject' ? 'problem' : 'suggestion',
    ]),
  ),
);

const recommendedRuleConfig = Object.freeze({
  'functional-parameters': 'error',
  'immutable-data': 'error',
  'no-class-inheritance': 'error',
  'no-classes': 'error',
  'no-conditional-statements': ['error', { allowReturningBranches: true }],
  'no-expression-statements': 'error',
  'no-let': ['error', { allowInForLoopInit: true }],
  'no-loop-statements': 'error',
  'no-mixed-types': 'error',
  'no-promise-reject': 'off',
  'no-return-void': 'error',
  'no-this-expressions': 'off',
  'no-throw-statements': ['error', { allowToRejectPromises: true }],
  'no-try-statements': 'off',
  'prefer-immutable-types': 'error',
  'prefer-property-signatures': 'error',
  'prefer-readonly-type': 'error',
  'prefer-tacit': 'warn',
  'readonly-type': 'error',
  'type-declaration-immutability': 'error',
});

// Upstream eslint-plugin-functional messageIds per rule. The Rust core tags
// each diagnostic with its messageId; the wrapper reports it (rendered through
// the `{{message}}` template so the displayed text stays our own copy and no
// upstream message strings are vendored). The upstream replay suite asserts
// these ids.
const ruleMessageIds = Object.freeze({
  'functional-parameters': [
    'restParam',
    'arguments',
    'paramCountAtLeastOne',
    'paramCountExactlyOne',
  ],
  'immutable-data': ['generic', 'object', 'array', 'map', 'set'],
  'no-class-inheritance': ['abstract', 'extends'],
  'no-classes': ['generic'],
  'no-conditional-statements': [
    'incompleteBranch',
    'incompleteIf',
    'incompleteSwitch',
    'unexpectedIf',
    'unexpectedSwitch',
  ],
  'no-expression-statements': ['generic'],
  'no-let': ['generic'],
  'no-loop-statements': ['generic'],
  'no-mixed-types': ['generic'],
  'no-promise-reject': ['generic'],
  'no-return-void': ['generic'],
  'no-this-expressions': ['generic'],
  'no-throw-statements': ['generic'],
  'no-try-statements': ['catch', 'finally'],
  'prefer-immutable-types': [
    'parameter',
    'returnType',
    'variable',
    'propertyImmutability',
    'propertyModifier',
    'propertyModifierSuggestion',
    'userDefined',
  ],
  'prefer-property-signatures': ['generic'],
  'prefer-readonly-type': ['array', 'implicit', 'property', 'tuple', 'type'],
  'prefer-tacit': ['generic', 'genericSuggestion'],
  'readonly-type': ['generic', 'keyword'],
  'type-declaration-immutability': ['Less', 'AtLeast', 'Exactly', 'AtMost', 'More', 'userDefined'],
});

function messagesForRule(ruleName) {
  const ids = ruleMessageIds[ruleName] ?? ['generic'];
  return Object.fromEntries(ids.map((messageId) => [messageId, '{{message}}']));
}

const implementedRuleNames = Object.freeze(implementedFunctionalRuleNames());
const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createFunctionalRule(ruleName)]),
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
    all: configFromRuleConfig(
      'all',
      Object.fromEntries(implementedRuleNames.map((rule) => [rule, 'error'])),
    ),
    recommended: configFromRuleConfig('recommended', recommendedRuleConfig),
    off: configFromRuleConfig(
      'off',
      Object.fromEntries(implementedRuleNames.map((rule) => [rule, 'off'])),
    ),
  },
});

plugin.implementedFunctionalRuleNames = implementedRuleNames;
plugin.scanFunctional = scanFunctional;

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

function createFunctionalRule(ruleName) {
  return {
    meta: {
      type: ruleTypes[ruleName],
      docs: {
        description: ruleDescriptions[ruleName],
        category: 'Functional Programming',
        recommended: recommendedRuleConfig[ruleName] !== 'off',
        url: `${DOCS_BASE}#${ruleName}`,
      },
      messages: messagesForRule(ruleName),
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

function stringOrStringArraySchema() {
  return {
    anyOf: [{ type: 'string' }, { type: 'array', items: { type: 'string' } }],
  };
}

function schemaForRule(ruleName) {
  if (ruleName === 'no-classes' || ruleName === 'no-class-inheritance') {
    return [
      {
        type: 'object',
        properties: {
          ignoreIdentifierPattern: stringOrStringArraySchema(),
          ignoreCodePattern: stringOrStringArraySchema(),
        },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'functional-parameters') {
    return [
      {
        type: 'object',
        properties: {
          allowRestParameter: { type: 'boolean' },
          allowArgumentsKeyword: { type: 'boolean' },
          enforceParameterCount: {
            oneOf: [
              { type: 'boolean', enum: [false] },
              { type: 'string', enum: ['atLeastOne', 'exactlyOne'] },
              {
                type: 'object',
                properties: {
                  count: { type: 'string', enum: ['atLeastOne', 'exactlyOne'] },
                  ignoreIIFE: { type: 'boolean' },
                  ignoreLambdaExpression: { type: 'boolean' },
                  ignoreGettersAndSetters: { type: 'boolean' },
                },
                additionalProperties: false,
              },
            ],
          },
          ignoreIdentifierPattern: stringOrStringArraySchema(),
          ignorePrefixSelector: stringOrStringArraySchema(),
        },
        additionalProperties: true,
      },
    ];
  }
  if (ruleName === 'no-let') {
    return [
      {
        type: 'object',
        properties: {
          allowInForLoopInit: { type: 'boolean' },
          allowInFunctions: { type: 'boolean' },
          ignoreIdentifierPattern: stringOrStringArraySchema(),
        },
        additionalProperties: true,
      },
    ];
  }
  if (ruleName === 'no-throw-statements') {
    return [
      {
        type: 'object',
        properties: {
          allowToRejectPromises: { type: 'boolean' },
        },
        additionalProperties: true,
      },
    ];
  }
  if (ruleName === 'no-try-statements') {
    return [
      {
        type: 'object',
        properties: {
          allowCatch: { type: 'boolean' },
          allowFinally: { type: 'boolean' },
        },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'readonly-type') {
    return [{ type: 'string', enum: ['generic', 'keyword'] }];
  }
  if (ruleName === 'prefer-property-signatures') {
    return [
      {
        type: 'object',
        properties: {
          ignoreIfReadonlyWrapped: { type: 'boolean' },
        },
        additionalProperties: false,
      },
    ];
  }
  if (ruleName === 'no-mixed-types') {
    return [
      {
        type: 'object',
        properties: {
          checkInterfaces: { type: 'boolean' },
          checkTypeLiterals: { type: 'boolean' },
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
  const filename = typeof context.filename === 'string' ? context.filename : 'file.ts';
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

  const diagnostics = scanFunctional(sourceText, filename, options);
  sourceCache.set(key, { sourceText, filename, diagnostics });
  return diagnostics;
}

function classRuleUsesIgnorePatterns(ruleName) {
  return ruleName === 'no-classes' || ruleName === 'no-class-inheritance';
}

function normalizeEnforceParameterCount(enforceOpt) {
  // enforceOpt can be: false | "atLeastOne" | "exactlyOne" | { count, ignoreIIFE, ... }
  if (enforceOpt === false) {
    return {
      enforceParameterCount: 'off',
      enforceCountIgnoreIife: true,
      enforceCountIgnoreGettersSetters: true,
      enforceCountIgnoreLambda: false,
    };
  }
  if (typeof enforceOpt === 'string') {
    return {
      enforceParameterCount: enforceOpt,
      enforceCountIgnoreIife: true,
      enforceCountIgnoreGettersSetters: true,
      enforceCountIgnoreLambda: false,
    };
  }
  if (enforceOpt && typeof enforceOpt === 'object') {
    const count = typeof enforceOpt.count === 'string' ? enforceOpt.count : 'atLeastOne';
    const ignoreIife = typeof enforceOpt.ignoreIIFE === 'boolean' ? enforceOpt.ignoreIIFE : true;
    const ignoreGettersSetter =
      typeof enforceOpt.ignoreGettersAndSetters === 'boolean'
        ? enforceOpt.ignoreGettersAndSetters
        : true;
    const ignoreLambda =
      typeof enforceOpt.ignoreLambdaExpression === 'boolean'
        ? enforceOpt.ignoreLambdaExpression
        : false;
    return {
      enforceParameterCount: count,
      enforceCountIgnoreIife: ignoreIife,
      enforceCountIgnoreGettersSetters: ignoreGettersSetter,
      enforceCountIgnoreLambda: ignoreLambda,
    };
  }
  // Default: atLeastOne, ignoreIIFE=true, ignoreGettersAndSetters=true
  return {
    enforceParameterCount: 'atLeastOne',
    enforceCountIgnoreIife: true,
    enforceCountIgnoreGettersSetters: true,
    enforceCountIgnoreLambda: false,
  };
}

// Parse an ignorePrefixSelector string/string[] into an array of method-name
// strings extracted from the narrow pattern
// `CallExpression[callee.property.name='NAME']`.
const PREFIX_SELECTOR_RE = /^CallExpression\[callee\.property\.name='([^']+)'\]$/;

function parsePrefixSelectorNames(selectorOpt) {
  if (!selectorOpt) return undefined;
  const selectors = Array.isArray(selectorOpt) ? selectorOpt : [selectorOpt];
  const names = [];
  for (const sel of selectors) {
    const match = typeof sel === 'string' ? PREFIX_SELECTOR_RE.exec(sel) : null;
    if (match) {
      names.push(match[1]);
    }
  }
  return names.length > 0 ? names : undefined;
}

function scanOptionsForRule(context, ruleName) {
  const raw = context.options?.[0];
  const options = raw && typeof raw === 'object' ? raw : {};

  let enforceParameterCount;
  let enforceCountIgnoreIife;
  let enforceCountIgnoreGettersSetters;
  let enforceCountIgnoreLambda;
  let ignoreIdentifierPatternForFunctional;
  let ignorePrefixSelectorNames;

  if (ruleName === 'functional-parameters') {
    const normalized = normalizeEnforceParameterCount(options.enforceParameterCount);
    enforceParameterCount = normalized.enforceParameterCount;
    enforceCountIgnoreIife = normalized.enforceCountIgnoreIife;
    enforceCountIgnoreGettersSetters = normalized.enforceCountIgnoreGettersSetters;
    enforceCountIgnoreLambda = normalized.enforceCountIgnoreLambda;
    ignoreIdentifierPatternForFunctional = options.ignoreIdentifierPattern;
    ignorePrefixSelectorNames = parsePrefixSelectorNames(options.ignorePrefixSelector);
  }

  return {
    ruleNames: [ruleName],
    allowRestParameter: ruleName === 'functional-parameters' && options.allowRestParameter === true,
    allowArgumentsKeyword:
      ruleName === 'functional-parameters' && options.allowArgumentsKeyword === true,
    allowLetInForLoopInit: ruleName === 'no-let' && options.allowInForLoopInit === true,
    allowInFunctions: ruleName === 'no-let' && options.allowInFunctions === true,
    allowThrowToRejectPromises:
      ruleName === 'no-throw-statements' && options.allowToRejectPromises === true,
    allowTryCatch: ruleName === 'no-try-statements' && options.allowCatch === true,
    allowTryFinally: ruleName === 'no-try-statements' && options.allowFinally === true,
    readonlyTypeMode:
      ruleName === 'readonly-type' && (raw === 'keyword' || raw === 'generic') ? raw : undefined,
    ignoreIfReadonlyWrapped:
      ruleName === 'prefer-property-signatures' && options.ignoreIfReadonlyWrapped === true,
    ignoreIdentifierPattern:
      classRuleUsesIgnorePatterns(ruleName) || ruleName === 'no-let'
        ? options.ignoreIdentifierPattern
        : ruleName === 'functional-parameters'
          ? ignoreIdentifierPatternForFunctional
          : undefined,
    ignoreCodePattern: classRuleUsesIgnorePatterns(ruleName)
      ? options.ignoreCodePattern
      : undefined,
    enforceParameterCount,
    enforceCountIgnoreIife,
    enforceCountIgnoreGettersSetters,
    enforceCountIgnoreLambda,
    ignorePrefixSelectorNames,
    checkInterfaces: ruleName === 'no-mixed-types' ? options.checkInterfaces !== false : undefined,
    checkTypeLiterals:
      ruleName === 'no-mixed-types' ? options.checkTypeLiterals !== false : undefined,
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
