'use strict';

// Oxlint plugin port of eslint-plugin-cypress (MIT).
// The JavaScript layer is only an Oxlint/NAPI adapter; parsing, Cypress chain
// analysis, variable tracking, and rule checks run in Rust through Oxc.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedCypressRuleNames, scanCypress } = require('./api.js');

const PLUGIN_NAME = 'cypress';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/cypress';
const diagnosticsCache = new WeakMap();

const commonGlobals = Object.freeze({
  cy: false,
  Cypress: false,
  expect: false,
  assert: false,
  chai: false,
  after: false,
  afterEach: false,
  before: false,
  beforeEach: false,
  context: false,
  describe: false,
  it: false,
  specify: false,
  test: false,
});

const messages = {
  'assertion-before-screenshot': {
    unexpected: 'Make an assertion on the page state before taking a screenshot',
  },
  'no-and': {
    unexpected:
      'Use .should() here; .and() is only allowed after .should(), .and(), or .contains().',
  },
  'no-assigning-return-values': {
    unexpected: 'Do not assign the return value of a Cypress command',
  },
  'no-async-before': {
    unexpected: 'Avoid using async functions with Cypress before / beforeEach functions',
  },
  'no-async-tests': {
    unexpected: 'Avoid using async functions with Cypress tests',
  },
  'no-chained-get': {
    unexpected: 'Avoid chaining multiple cy.get() calls',
  },
  'no-debug': {
    unexpected: 'Do not use cy.debug command',
  },
  'no-force': {
    unexpected: 'Do not use force on click and type calls',
  },
  'no-pause': {
    unexpected: 'Do not use cy.pause command',
  },
  'no-unnecessary-waiting': {
    unexpected: 'Do not wait for arbitrary time periods',
  },
  'no-xpath': {
    unexpected:
      'cy.xpath() is deprecated and unsupported. Consider using cy.get() with appropriate selectors instead.',
  },
  'require-data-selectors': {
    unexpected: 'use data-* attribute selectors instead of classes or tag names',
  },
  'unsafe-to-chain-command': {
    unexpected:
      'It is unsafe to chain further commands that rely on the subject after this command. It is best to split the chain, chaining again from `cy.` in a next command line.',
  },
};

const ruleDescriptions = {
  'assertion-before-screenshot': 'require screenshots to be preceded by an assertion',
  'no-and': 'enforce .should() over .and() for starting assertion chains',
  'no-assigning-return-values': 'disallow assigning return values of `cy` calls',
  'no-async-before': 'disallow using `async`/`await` in Cypress `before` methods',
  'no-async-tests': 'disallow using `async`/`await` in Cypress test cases',
  'no-chained-get': 'disallow chain of `cy.get()` calls',
  'no-debug': 'disallow using `cy.debug()` calls',
  'no-force': 'disallow using `force: true` with action commands',
  'no-pause': 'disallow using `cy.pause()` calls',
  'no-unnecessary-waiting': 'disallow waiting for arbitrary time periods',
  'no-xpath': 'disallow using `cy.xpath()` calls',
  'require-data-selectors': 'require `data-*` attribute selectors',
  'unsafe-to-chain-command': 'disallow actions within chains',
};

const ruleTypes = {
  'assertion-before-screenshot': 'problem',
  'no-and': 'suggestion',
  'no-assigning-return-values': 'problem',
  'no-async-before': 'problem',
  'no-async-tests': 'problem',
  'no-chained-get': 'problem',
  'no-debug': 'suggestion',
  'no-force': 'suggestion',
  'no-pause': 'suggestion',
  'no-unnecessary-waiting': 'problem',
  'no-xpath': 'problem',
  'require-data-selectors': 'suggestion',
  'unsafe-to-chain-command': 'problem',
};

const implementedRuleNames = Object.freeze(implementedCypressRuleNames());
const recommendedRuleNames = Object.freeze([
  'no-assigning-return-values',
  'no-unnecessary-waiting',
  'no-async-tests',
  'unsafe-to-chain-command',
]);

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createCypressRule(ruleName)]),
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
    globals: {
      name: `${PLUGIN_NAME}/globals`,
      plugins: [PLUGIN_NAME],
      languageOptions: {
        globals: commonGlobals,
      },
    },
    recommended: {
      name: `${PLUGIN_NAME}/recommended`,
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        recommendedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
      ),
      languageOptions: {
        globals: commonGlobals,
      },
    },
    'recommended-legacy': {
      plugins: [PLUGIN_NAME],
      env: {
        'cypress/globals': true,
      },
      rules: Object.fromEntries(
        recommendedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
      ),
    },
  },
});

plugin.implementedCypressRuleNames = implementedRuleNames;
plugin.scanCypress = scanCypress;

function createCypressRule(ruleName) {
  const meta = {
    type: ruleTypes[ruleName],
    docs: {
      description: ruleDescriptions[ruleName],
      category: 'Possible Errors',
      recommended: recommendedRuleNames.includes(ruleName),
      url: `${DOCS_BASE}#${ruleName}`,
    },
    messages: messages[ruleName],
    schema: schemaForRule(ruleName),
  };

  if (ruleName === 'no-and') {
    meta.fixable = 'code';
  }

  if (ruleName === 'no-xpath') {
    meta.deprecated = {
      message:
        'The underlying @cypress/xpath package was deprecated and removed in 2023; migrate to supported API calls.',
    };
  }

  return {
    meta,
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
  if (ruleName !== 'unsafe-to-chain-command') {
    return [];
  }

  return [
    {
      title: ruleName,
      description: ruleDescriptions[ruleName],
      type: 'object',
      properties: {
        methods: {
          type: 'array',
          description: 'An additional list of methods to check for unsafe chaining.',
          default: [],
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

  const byteToUtf16 = createByteToUtf16Mapper(sourceText);
  const diagnostics = scanCypress(sourceText, filename, options).map((diagnostic) =>
    mapDiagnosticFix(diagnostic, byteToUtf16),
  );
  sourceCache.set(key, { sourceText, filename, diagnostics });
  return diagnostics;
}

function scanOptionsForRule(context, ruleName) {
  if (ruleName !== 'unsafe-to-chain-command') {
    return { unsafeToChainMethods: [] };
  }

  return {
    unsafeToChainMethods: normalizeUnsafeMethods(context.options?.[0]?.methods),
  };
}

function normalizeUnsafeMethods(methods) {
  if (!Array.isArray(methods)) {
    return [];
  }
  return methods
    .map((method) => {
      if (typeof method === 'string') {
        return method;
      }
      if (method instanceof RegExp) {
        return method.source;
      }
      return null;
    })
    .filter((method) => typeof method === 'string' && method.length > 0);
}

function mapDiagnosticFix(diagnostic, byteToUtf16) {
  if (!diagnostic.fix) {
    return diagnostic;
  }

  return {
    ...diagnostic,
    fix: {
      start: byteToUtf16(diagnostic.fix.start),
      end: byteToUtf16(diagnostic.fix.end),
      replacement: diagnostic.fix.replacement,
    },
  };
}

function reportDiagnostic(context, diagnostic) {
  const descriptor = {
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
  };

  if (diagnostic.fix) {
    descriptor.fix = (fixer) =>
      fixer.replaceTextRange(
        [diagnostic.fix.start, diagnostic.fix.end],
        diagnostic.fix.replacement,
      );
  }

  context.report(descriptor);
}

function createByteToUtf16Mapper(sourceText) {
  const nonAsciiSpans = [];
  let byteOffset = 0;
  let utf16Offset = 0;

  while (utf16Offset < sourceText.length) {
    const codePoint = sourceText.codePointAt(utf16Offset);
    if (codePoint === undefined) {
      break;
    }

    const utf16Length = codePoint > 0xffff ? 2 : 1;
    const byteLength = utf8ByteLength(codePoint);
    const byteEnd = byteOffset + byteLength;
    const utf16End = utf16Offset + utf16Length;

    if (byteLength !== utf16Length) {
      nonAsciiSpans.push({
        byteStart: byteOffset,
        byteEnd,
        utf16Start: utf16Offset,
        deltaAfter: byteEnd - utf16End,
      });
    }

    byteOffset = byteEnd;
    utf16Offset = utf16End;
  }

  if (nonAsciiSpans.length === 0) {
    return (offset) => clampOffset(offset, sourceText.length);
  }

  const totalBytes = byteOffset;
  return (offset) => {
    const clampedByteOffset = clampOffset(offset, totalBytes);
    let low = 0;
    let high = nonAsciiSpans.length;

    while (low < high) {
      const mid = Math.floor((low + high) / 2);
      if (nonAsciiSpans[mid].byteEnd <= clampedByteOffset) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }

    const nextSpan = nonAsciiSpans[low];
    if (
      nextSpan &&
      clampedByteOffset >= nextSpan.byteStart &&
      clampedByteOffset < nextSpan.byteEnd
    ) {
      return nextSpan.utf16Start;
    }

    const previousSpan = nonAsciiSpans[low - 1];
    const delta = previousSpan?.deltaAfter ?? 0;
    return clampOffset(clampedByteOffset - delta, sourceText.length);
  };
}

function utf8ByteLength(codePoint) {
  if (codePoint <= 0x7f) {
    return 1;
  }
  if (codePoint <= 0x7ff) {
    return 2;
  }
  if (codePoint <= 0xffff) {
    return 3;
  }
  return 4;
}

function clampOffset(offset, max) {
  if (!Number.isFinite(offset)) {
    return 0;
  }
  if (offset <= 0) {
    return 0;
  }
  if (offset >= max) {
    return max;
  }
  return Math.trunc(offset);
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
module.exports.implementedCypressRuleNames = implementedRuleNames;
module.exports.scanCypress = scanCypress;
