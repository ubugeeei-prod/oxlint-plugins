'use strict';

// Oxlint plugin port of @e18e/eslint-plugin (MIT).
// The JavaScript layer adapts Oxlint's ESLint-compatible plugin API. Parsing,
// AST matching, and fix range calculation run in Rust through Oxc.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedE18eRuleNames, scanE18e } = require('./api.js');

const PLUGIN_NAME = 'e18e';
const DOCS_BASE = 'https://github.com/e18e/eslint-plugin';
const diagnosticsCache = new WeakMap();

const modernizationRuleConfig = Object.freeze({
  'prefer-array-at': 'error',
  'prefer-array-fill': 'error',
  'prefer-includes': 'error',
  'prefer-array-to-reversed': 'error',
  'prefer-array-to-sorted': 'error',
  'prefer-array-to-spliced': 'error',
  'prefer-nullish-coalescing': 'error',
  'prefer-object-has-own': 'error',
  'prefer-spread-syntax': 'error',
  'prefer-url-canparse': 'error',
});

const moduleReplacementsRuleConfig = Object.freeze({
  'ban-dependencies': 'error',
});

const performanceImprovementsRuleConfig = Object.freeze({
  'prefer-array-from-map': 'error',
  'prefer-timer-args': 'error',
  'prefer-date-now': 'error',
  'prefer-regex-test': 'error',
  'prefer-array-some': 'error',
  'prefer-static-regex': 'error',
  'prefer-string-fromcharcode': 'error',
});

const recommendedRuleConfig = Object.freeze({
  ...modernizationRuleConfig,
  ...moduleReplacementsRuleConfig,
  ...performanceImprovementsRuleConfig,
});

const defaultBannedDependencies = Object.freeze([
  {
    moduleName: 'lodash.merge',
    messageId: 'documentedReplacement',
    replacement: 'deepmerge-ts',
    url: 'https://github.com/es-tooling/module-replacements',
  },
  {
    moduleName: 'lodash.clonedeep',
    messageId: 'documentedReplacement',
    replacement: 'structuredClone',
    url: 'https://github.com/es-tooling/module-replacements',
  },
  {
    moduleName: 'left-pad',
    messageId: 'removalReplacement',
    description: 'This module is no longer needed in modern JavaScript.',
  },
]);

const messages = Object.freeze({
  'prefer-array-at': {
    preferAt: 'Use .at(-1) instead of [{{array}}.length - 1]',
  },
  'prefer-array-fill': {
    preferFillArrayFrom:
      'Use Array.from({length: {{length}}).fill({{value}}) instead of Array.from with a constant callback',
    preferFillSpreadMap: 'Use Array({{length}}).fill({{value}}) instead of spread Array with map',
  },
  'prefer-array-from-map': {
    preferArrayFrom:
      'Use Array.from({{iterable}}, {{mapper}}) instead of [...{{iterable}}].map({{mapper}}) to avoid creating an intermediate array',
  },
  'prefer-includes': {
    preferIncludes: 'Use .includes() instead of indexOf() comparison',
  },
  'prefer-array-to-reversed': {
    preferToReversed: 'Use {{array}}.toReversed() instead of copying and reversing',
  },
  'prefer-array-to-sorted': {
    preferToSorted: 'Use {{array}}.toSorted() instead of copying and sorting',
  },
  'prefer-array-to-spliced': {
    preferToSpliced: 'Use {{array}}.toSpliced() instead of copying and splicing',
  },
  'prefer-exponentiation-operator': {
    preferExponentiation: 'Use the ** operator instead of Math.pow()',
  },
  'prefer-nullish-coalescing': {
    preferNullishCoalescing: 'Use nullish coalescing operator (??) instead of verbose null check',
    preferNullishCoalescingAssignment:
      'Use nullish coalescing assignment (??=) instead of verbose null check',
  },
  'prefer-object-has-own': {
    preferObjectHasOwn: 'Use Object.hasOwn() instead of hasOwnProperty',
  },
  'prefer-spread-syntax': {
    preferSpreadArray: 'Use spread syntax [...arr, ...other] instead of arr.concat(other)',
    preferSpreadArrayFrom:
      'Use spread syntax [...iterable] instead of Array.from(iterable) when no mapper function is provided',
    preferSpreadObject: 'Use spread syntax {...a, ...b} instead of Object.assign({}, a, b)',
    preferSpreadFunction: 'Use spread syntax fn(...args) instead of fn.apply(null/undefined, args)',
  },
  'prefer-url-canparse': {
    preferCanParse: 'Use URL.canParse() instead of try-catch for URL validation',
    replaceWithCanParse: 'Replace with URL.canParse()',
  },
  'no-indexof-equality': {
    preferDirectAccess:
      'Use direct array access `{{array}}[{{index}}] === {{item}}` instead of `indexOf() === {{index}}`',
    preferStartsWith: 'Use `.startsWith()` instead of `indexOf() === 0` for strings',
  },
  'prefer-timer-args': {
    preferArgs:
      'Pass function and arguments directly to timer function to avoid allocating an extra function',
  },
  'prefer-date-now': {
    preferDateNow: 'Use Date.now() instead of creating a Date object.',
  },
  'prefer-regex-test': {
    preferTest: 'Prefer `{{regex}}.test({{string}})` over `{{original}}` for boolean checks',
  },
  'prefer-array-some': {
    preferArraySome:
      'Use Array.some() instead of Array.find() and Array.filter().length checks when checking for element existence',
  },
  'prefer-static-regex': {
    preferStatic:
      'Move this regular expression to module scope to avoid re-compilation on every call.',
  },
  'prefer-inline-equality': {
    preferEquality:
      'Avoid creating a temporary array just to call `.includes()`. Use equality checks instead.',
  },
  'prefer-string-fromcharcode': {
    preferFromCharCode:
      'String.fromCharCode is faster than String.fromCodePoint for code points below 0x10000.',
  },
  'prefer-includes-over-regex-test': {
    preferIncludes: 'Use `String.prototype.includes()` instead of a simple regex test.',
    preferStartsWith: 'Use `String.prototype.startsWith()` instead of a simple regex test.',
    preferEndsWith: 'Use `String.prototype.endsWith()` instead of a simple regex test.',
    preferEquals: 'Use equality instead of a simple regex test.',
  },
  'no-delete-property': {
    noDeleteProperty:
      '`delete` forces V8 into a slow dictionary representation. Set the value to `undefined` if absence-vs-undefined does not matter, or use `Map` for a dynamic key-value collection.',
    replaceWithUndefined: 'Replace with assignment to undefined',
  },
  'no-spread-in-reduce': {
    noSpreadInReduce: 'Avoid object/array spread inside reduce callbacks.',
  },
  'prefer-static-collator': {
    preferStaticCollator:
      'Move this Intl.Collator construction to module scope to avoid re-creating it on every call.',
  },
  'ban-dependencies': {
    nativeReplacement:
      '"{{name}}" should be replaced with native functionality. You can instead use {{replacement}}. Read more here: {{url}}',
    documentedReplacement:
      '"{{name}}" should be replaced with an alternative package. In your project, we recommend {{replacement}}. Read more here: {{url}}',
    simpleReplacement: '"{{name}}" should be replaced with inline/local logic.{{description}}',
    removalReplacement: '"{{name}}" is flagged as no longer needed. {{description}}',
  },
});

const ruleDescriptions = Object.freeze({
  'prefer-array-at': 'Prefer Array.prototype.at() over length-based indexing',
  'prefer-array-fill': 'Prefer Array.prototype.fill() over Array.from or map with constant values',
  'prefer-array-from-map': 'Prefer Array.from(iterable, mapper) over [...iterable].map(mapper)',
  'prefer-includes': 'Prefer .includes() over indexOf() comparisons for arrays and strings',
  'prefer-array-to-reversed':
    'Prefer Array.prototype.toReversed() over copying and reversing arrays',
  'prefer-array-to-sorted': 'Prefer Array.prototype.toSorted() over copying and sorting arrays',
  'prefer-array-to-spliced': 'Prefer Array.prototype.toSpliced() over copying and splicing arrays',
  'prefer-exponentiation-operator': 'Prefer the exponentiation operator ** over Math.pow()',
  'prefer-nullish-coalescing':
    'Prefer nullish coalescing operator (?? and ??=) over verbose null checks',
  'prefer-object-has-own':
    'Prefer Object.hasOwn() over Object.prototype.hasOwnProperty.call() and obj.hasOwnProperty()',
  'prefer-spread-syntax':
    'Prefer spread syntax over Array.concat(), Array.from(), Object.assign({}, ...), and Function.apply()',
  'prefer-url-canparse': 'Prefer URL.canParse() over try-catch blocks for URL validation',
  'no-indexof-equality': 'Prefer optimized alternatives to indexOf() equality checks',
  'prefer-timer-args': 'Prefer passing function and arguments directly to setTimeout/setInterval',
  'prefer-date-now': 'Prefer Date.now() over Date object allocation',
  'prefer-regex-test': 'Prefer RegExp.test() over String.match() and RegExp.exec()',
  'prefer-array-some': 'Prefer Array.some() over Array.find() and Array.filter().length checks',
  'prefer-static-regex': 'Prefer defining regular expressions at module scope',
  'prefer-inline-equality': 'Prefer inline equality checks over temporary object creation',
  'prefer-string-fromcharcode':
    'Prefer String.fromCharCode() over String.fromCodePoint() for BMP code points',
  'prefer-includes-over-regex-test': 'Prefer string methods over simple regex tests',
  'no-delete-property': 'Disallow delete on properties',
  'no-spread-in-reduce': 'Disallow object/array spread inside reduce callbacks',
  'prefer-static-collator': 'Prefer defining Intl.Collator at module scope',
  'ban-dependencies': 'Disallow dependencies in favor of faster or native alternatives',
});

const fixableRules = new Set([
  'prefer-array-at',
  'prefer-array-fill',
  'prefer-array-from-map',
  'prefer-includes',
  'prefer-array-to-reversed',
  'prefer-array-to-sorted',
  'prefer-array-to-spliced',
  'prefer-exponentiation-operator',
  'prefer-nullish-coalescing',
  'prefer-object-has-own',
  'prefer-spread-syntax',
  'prefer-url-canparse',
  'no-indexof-equality',
  'prefer-timer-args',
  'prefer-date-now',
  'prefer-regex-test',
  'prefer-array-some',
  'prefer-inline-equality',
  'prefer-string-fromcharcode',
  'prefer-includes-over-regex-test',
  'no-delete-property',
]);

const implementedRuleNames = Object.freeze(implementedE18eRuleNames());
const rules = Object.freeze(
  Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, createE18eRule(ruleName)])),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 0])),
  configs: {
    modernization: configFromRuleConfig('modernization', modernizationRuleConfig),
    'module-replacements': configFromRuleConfig(
      'module-replacements',
      moduleReplacementsRuleConfig,
    ),
    'performance-improvements': configFromRuleConfig(
      'performance-improvements',
      performanceImprovementsRuleConfig,
    ),
    recommended: configFromRuleConfig('recommended', recommendedRuleConfig),
  },
});

plugin.implementedE18eRuleNames = implementedRuleNames;
plugin.scanE18e = scanE18e;

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

function createE18eRule(ruleName) {
  return {
    meta: {
      type: 'suggestion',
      docs: {
        description: ruleDescriptions[ruleName],
        category: 'Performance',
        recommended: recommendedRuleConfig[ruleName] !== undefined,
        url: `${DOCS_BASE}/blob/main/src/rules/${ruleName}.ts`,
      },
      fixable: fixableRules.has(ruleName) ? 'code' : undefined,
      hasSuggestions: ruleName === 'prefer-url-canparse' || ruleName === 'no-delete-property',
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
  if (ruleName !== 'ban-dependencies') {
    return [];
  }
  return [
    {
      type: 'object',
      properties: {
        presets: { type: 'array', items: { type: 'string' } },
        modules: { type: 'array', items: { type: 'string' } },
        allowed: { type: 'array', items: { type: 'string' } },
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

  const diagnostics = scanE18e(sourceText, filename, options);
  sourceCache.set(key, { sourceText, filename, diagnostics });
  return diagnostics;
}

function scanOptionsForRule(context, ruleName) {
  const options = { ruleNames: [ruleName] };
  if (ruleName === 'ban-dependencies') {
    options.bannedDependencies = bannedDependenciesForOptions(context.options);
  }
  return options;
}

function bannedDependenciesForOptions(rawOptions) {
  const options = mergeOptions(rawOptions);
  const allowed = new Set(
    Array.isArray(options.allowed)
      ? options.allowed.filter((value) => typeof value === 'string')
      : [],
  );
  const useDefaults = !Array.isArray(options.presets) || options.presets.length > 0;
  const banned = useDefaults ? [...defaultBannedDependencies] : [];

  if (Array.isArray(options.modules)) {
    for (const moduleName of options.modules) {
      if (typeof moduleName === 'string' && moduleName.length > 0) {
        banned.push({
          moduleName,
          messageId: 'removalReplacement',
          description: 'This module is disallowed and should be replaced with an alternative.',
        });
      }
    }
  }

  return banned.filter((entry) => {
    for (const allowedName of allowed) {
      if (entry.moduleName === allowedName || entry.moduleName.startsWith(`${allowedName}/`)) {
        return false;
      }
    }
    return true;
  });
}

function mergeOptions(options) {
  if (!Array.isArray(options)) {
    return {};
  }
  return Object.assign({}, ...options.filter((option) => option && typeof option === 'object'));
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
module.exports.implementedE18eRuleNames = implementedRuleNames;
module.exports.scanE18e = scanE18e;
