'use strict';

// Oxlint plugin port of @unocss/eslint-plugin (MIT).
// The JavaScript layer is only an Oxlint/NAPI adapter; class token scanning,
// ordering, blocklist, and class-compile checks run in Rust through Oxc.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedUnocssRuleNames, scanUnocss } = require('./api.js');

const PLUGIN_NAME = '@unocss';
const DOCS_BASE = 'https://unocss.dev/integrations/eslint';
const diagnosticsCache = new WeakMap();

const messages = {
  blocklist: {
    'in-blocklist': '"{{name}}" is in blocklist{{reason}}',
  },
  'enforce-class-compile': {
    missing: 'prefix: `{{prefix}}` is missing',
  },
  order: {
    'invalid-order': 'UnoCSS utilities are not ordered',
  },
  'order-attributify': {
    'invalid-order': 'UnoCSS attributes are not ordered',
  },
};

const ruleDescriptions = {
  blocklist: 'disallow utilities configured in the UnoCSS blocklist',
  'enforce-class-compile': 'enforce the UnoCSS class compilation prefix',
  order: 'enforce sorted UnoCSS utilities in class strings',
  'order-attributify': 'enforce sorted UnoCSS attributify attributes',
};

const ruleTypes = {
  blocklist: 'problem',
  'enforce-class-compile': 'problem',
  order: 'layout',
  'order-attributify': 'layout',
};

const implementedRuleNames = Object.freeze(implementedUnocssRuleNames());
const recommendedRuleNames = Object.freeze(['order', 'order-attributify']);
// `blocklist` only reports; it never emits a fix, so it is not `fixable`.
const fixableRuleNames = Object.freeze(['order', 'order-attributify', 'enforce-class-compile']);

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createUnocssRule(ruleName)]),
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
      rules: Object.fromEntries(
        recommendedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'warn']),
      ),
    },
    all: {
      name: `${PLUGIN_NAME}/all`,
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'warn']),
      ),
    },
    off: {
      name: `${PLUGIN_NAME}/off`,
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'off']),
      ),
    },
  },
});

plugin.implementedUnocssRuleNames = implementedRuleNames;
plugin.scanUnocss = scanUnocss;

function createUnocssRule(ruleName) {
  return {
    meta: {
      type: ruleTypes[ruleName],
      docs: {
        description: ruleDescriptions[ruleName],
        category: ruleTypes[ruleName] === 'layout' ? 'Stylistic Issues' : 'Possible Errors',
        recommended: recommendedRuleNames.includes(ruleName),
        url: `${DOCS_BASE}#rules`,
      },
      fixable: fixableRuleNames.includes(ruleName) ? 'code' : undefined,
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
  if (ruleName === 'order') {
    return [
      {
        type: 'object',
        properties: {
          unoFunctions: {
            type: 'array',
            items: { type: 'string' },
          },
          unoVariables: {
            type: 'array',
            items: { type: 'string' },
          },
        },
        additionalProperties: false,
      },
    ];
  }

  if (ruleName === 'enforce-class-compile') {
    return [
      {
        type: 'object',
        properties: {
          prefix: {
            type: 'string',
          },
          enableFix: {
            type: 'boolean',
          },
        },
        additionalProperties: false,
      },
    ];
  }

  if (ruleName === 'blocklist') {
    return [
      {
        type: 'object',
        properties: {
          blocklist: {
            type: 'array',
          },
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
  const filename = typeof context.filename === 'string' ? context.filename : 'file.tsx';
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
  const diagnostics = scanUnocss(sourceText, filename, options).map((diagnostic) =>
    mapDiagnosticFix(diagnostic, byteToUtf16),
  );
  sourceCache.set(key, { sourceText, filename, diagnostics });
  return diagnostics;
}

function scanOptionsForRule(context, ruleName) {
  const option = context.options?.[0] || {};
  const settings = context.settings?.unocss || {};
  return {
    unoFunctions: ruleName === 'order' ? normalizeStringArray(option.unoFunctions) : [],
    unoVariables: ruleName === 'order' ? normalizeStringArray(option.unoVariables) : [],
    blocklist:
      ruleName === 'blocklist' ? normalizeBlocklist(option.blocklist ?? settings.blocklist) : [],
    classCompilePrefix:
      ruleName === 'enforce-class-compile' && typeof option.prefix === 'string'
        ? option.prefix
        : undefined,
    classCompileEnableFix:
      ruleName === 'enforce-class-compile' && typeof option.enableFix === 'boolean'
        ? option.enableFix
        : undefined,
  };
}

function normalizeStringArray(values) {
  if (!Array.isArray(values) || values.length === 0) {
    return [];
  }
  return values.filter((value) => typeof value === 'string' && value.length > 0);
}

function normalizeBlocklist(values) {
  if (!Array.isArray(values)) {
    return [];
  }

  return values
    .map((value) => {
      if (typeof value === 'string') {
        return { name: value };
      }
      if (Array.isArray(value) && typeof value[0] === 'string') {
        return {
          name: value[0],
          reason: typeof value[1]?.message === 'string' ? `: ${value[1].message}` : undefined,
        };
      }
      if (value && typeof value.name === 'string') {
        return {
          name: value.name,
          reason: typeof value.reason === 'string' ? `: ${value.reason}` : undefined,
        };
      }
      return null;
    })
    .filter(Boolean);
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

  const data = {};
  if (diagnostic.name != null) {
    data.name = diagnostic.name;
  }
  if (diagnostic.reason != null) {
    data.reason = diagnostic.reason;
  }
  if (diagnostic.prefix != null) {
    data.prefix = diagnostic.prefix;
  }
  if (Object.keys(data).length > 0) {
    descriptor.data = data;
  }

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
module.exports.implementedUnocssRuleNames = implementedRuleNames;
module.exports.scanUnocss = scanUnocss;
