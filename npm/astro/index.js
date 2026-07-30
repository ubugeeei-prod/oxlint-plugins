'use strict';

// Oxlint plugin port of eslint-plugin-astro (MIT).
// The Rust core segments and parses Astro frontmatter with Oxc. React rules
// are intentionally outside this package's scope.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedAstroRuleNames, scanAstro } = require('./api.js');

const PLUGIN_NAME = 'astro';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/astro';
const diagnosticsCache = new WeakMap();

const descriptions = Object.freeze({
  'no-deprecated-astro-canonicalurl': 'disallow using deprecated `Astro.canonicalURL`',
  'no-deprecated-astro-fetchcontent': 'disallow using deprecated `Astro.fetchContent()`',
  'no-deprecated-getentrybyslug': 'disallow using deprecated `getEntryBySlug()`',
});

const messages = Object.freeze({
  'no-deprecated-astro-canonicalurl': {
    deprecated: "'Astro.canonicalURL' is deprecated. Use 'Astro.url' helper instead.",
  },
  'no-deprecated-astro-fetchcontent': {
    deprecated: "'Astro.fetchContent()' is deprecated. Use 'Astro.glob()' instead.",
  },
  'no-deprecated-getentrybyslug': {
    deprecated: "'getEntryBySlug()' is deprecated. Use 'getEntry()' instead.",
  },
});

const implementedRuleNames = Object.freeze(implementedAstroRuleNames());
const rules = Object.freeze(
  Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, createAstroRule(ruleName)])),
);
const recommendedRules = Object.freeze(
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
    recommended: {
      name: `${PLUGIN_NAME}/recommended`,
      plugins: [PLUGIN_NAME],
      files: ['**/*.astro'],
      rules: { ...recommendedRules },
    },
  },
});

plugin.implementedAstroRuleNames = implementedRuleNames;
plugin.scanAstro = scanAstro;

function createAstroRule(ruleName) {
  const meta = {
    type: 'problem',
    docs: {
      description: descriptions[ruleName],
      category: 'Possible Errors',
      recommended: true,
      url: `${DOCS_BASE}#${ruleName}`,
    },
    messages: messages[ruleName],
    schema: [],
  };
  if (ruleName === 'no-deprecated-astro-fetchcontent') {
    meta.fixable = 'code';
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

function diagnosticsForRule(context, ruleName) {
  const sourceCode = context.sourceCode ?? context.getSourceCode?.() ?? {};
  const sourceText =
    typeof sourceCode.getText === 'function'
      ? sourceCode.getText()
      : typeof sourceCode.text === 'string'
        ? sourceCode.text
        : '';
  const filename = typeof context.filename === 'string' ? context.filename : 'file.astro';
  const frontmatterOnly = !startsWithFrontmatterDelimiter(sourceText);
  let byRule = diagnosticsCache.get(sourceCode);
  if (!byRule) {
    byRule = new Map();
    diagnosticsCache.set(sourceCode, byRule);
  }
  const cached = byRule.get(ruleName);
  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const byteToUtf16 = createByteToUtf16Mapper(sourceText);
  const diagnostics = scanAstro(sourceText, filename, {
    ruleNames: [ruleName],
    frontmatterOnly,
  }).map((diagnostic) => mapDiagnosticFix(diagnostic, byteToUtf16));
  byRule.set(ruleName, { sourceText, filename, diagnostics });
  return diagnostics;
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

function startsWithFrontmatterDelimiter(sourceText) {
  return /^(?:\uFEFF)?---[ \t]*(?:\r\n|[\n\r\u2028\u2029])/.test(sourceText);
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
    const clamped = clampOffset(offset, totalBytes);
    let low = 0;
    let high = nonAsciiSpans.length;
    while (low < high) {
      const mid = Math.floor((low + high) / 2);
      if (nonAsciiSpans[mid].byteEnd <= clamped) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    const next = nonAsciiSpans[low];
    if (next && clamped >= next.byteStart && clamped < next.byteEnd) {
      return next.utf16Start;
    }
    const delta = nonAsciiSpans[low - 1]?.deltaAfter ?? 0;
    return clampOffset(clamped - delta, sourceText.length);
  };
}

function utf8ByteLength(codePoint) {
  if (codePoint <= 0x7f) return 1;
  if (codePoint <= 0x7ff) return 2;
  if (codePoint <= 0xffff) return 3;
  return 4;
}

function clampOffset(offset, maximum) {
  if (!Number.isFinite(offset) || offset <= 0) return 0;
  if (offset >= maximum) return maximum;
  return Math.trunc(offset);
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedAstroRuleNames = implementedRuleNames;
module.exports.scanAstro = scanAstro;
