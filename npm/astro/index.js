'use strict';

const { readFileSync } = require('node:fs');

// Oxlint plugin port of eslint-plugin-astro (MIT).
// The Rust core parses frontmatter with Oxc and conservatively segments
// template expressions, attributes, and element bodies. React rules are
// intentionally outside this package's scope.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedAstroRuleNames, scanAstro } = require('./api.js');

const PLUGIN_NAME = 'astro';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/astro';
const diagnosticsCache = new WeakMap();

const descriptions = Object.freeze({
  'no-deprecated-astro-canonicalurl': 'disallow using deprecated `Astro.canonicalURL`',
  'no-deprecated-astro-fetchcontent': 'disallow using deprecated `Astro.fetchContent()`',
  'no-deprecated-astro-resolve': 'disallow using deprecated `Astro.resolve()`',
  'no-deprecated-getentrybyslug': 'disallow using deprecated `getEntryBySlug()`',
  'no-set-html-directive': 'disallow use of `set:html` to prevent XSS attacks',
  'no-set-text-directive': 'disallow use of `set:text`',
  'prefer-class-list-directive':
    'require `class:list` directives instead of `class` with expressions',
});
const categories = Object.freeze({
  'no-deprecated-astro-canonicalurl': 'Possible Errors',
  'no-deprecated-astro-fetchcontent': 'Possible Errors',
  'no-deprecated-astro-resolve': 'Possible Errors',
  'no-deprecated-getentrybyslug': 'Possible Errors',
  'no-set-html-directive': 'Security Vulnerability',
  'no-set-text-directive': 'Best Practices',
  'prefer-class-list-directive': 'Stylistic Issues',
});

const messages = Object.freeze({
  'no-deprecated-astro-canonicalurl': {
    deprecated: "'Astro.canonicalURL' is deprecated. Use 'Astro.url' helper instead.",
  },
  'no-deprecated-astro-fetchcontent': {
    deprecated: "'Astro.fetchContent()' is deprecated. Use 'Astro.glob()' instead.",
  },
  'no-deprecated-astro-resolve': {
    deprecated: "'Astro.resolve()' is deprecated.",
  },
  'no-deprecated-getentrybyslug': {
    deprecated: "'getEntryBySlug()' is deprecated. Use 'getEntry()' instead.",
  },
  'no-set-html-directive': {
    unexpected: '`set:html` can lead to XSS attack.',
  },
  'no-set-text-directive': {
    disallow: "Don't use `set:text`.",
  },
  'prefer-class-list-directive': {
    unexpected: "Unexpected `class` using expression. Use 'class:list' instead.",
  },
});

const implementedRuleNames = Object.freeze(implementedAstroRuleNames());
const recommendedRuleNames = new Set([
  'no-deprecated-astro-canonicalurl',
  'no-deprecated-astro-fetchcontent',
  'no-deprecated-astro-resolve',
  'no-deprecated-getentrybyslug',
]);
const rules = Object.freeze(
  Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, createAstroRule(ruleName)])),
);
const recommendedRules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames
      .filter((ruleName) => recommendedRuleNames.has(ruleName))
      .map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
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
  const recommended = recommendedRuleNames.has(ruleName);
  const suggestion = new Set([
    'no-set-html-directive',
    'no-set-text-directive',
    'prefer-class-list-directive',
  ]).has(ruleName);
  const meta = {
    type: suggestion ? 'suggestion' : 'problem',
    docs: {
      description: descriptions[ruleName],
      category: categories[ruleName],
      recommended,
      url: `${DOCS_BASE}#${ruleName}`,
    },
    messages: messages[ruleName],
    schema: [],
  };
  if (
    ruleName === 'no-deprecated-astro-fetchcontent' ||
    ruleName === 'no-set-text-directive' ||
    ruleName === 'prefer-class-list-directive'
  ) {
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
  const physicalSource = readPhysicalSource(context, filename, sourceText);
  const scanSource = physicalSource?.sourceText ?? sourceText;
  const frontmatterOnly =
    physicalSource === null &&
    !startsWithFrontmatterDelimiter(sourceText) &&
    !looksLikeAstroTemplate(sourceText);
  let byRule = diagnosticsCache.get(sourceCode);
  if (!byRule) {
    byRule = new Map();
    diagnosticsCache.set(sourceCode, byRule);
  }
  const cached = byRule.get(ruleName);
  if (
    cached &&
    cached.sourceText === sourceText &&
    cached.scanSource === scanSource &&
    cached.filename === filename
  ) {
    return cached.diagnostics;
  }

  const byteToUtf16 = createByteToUtf16Mapper(scanSource);
  const diagnostics = scanAstro(scanSource, filename, {
    ruleNames: [ruleName],
    frontmatterOnly,
  }).map((diagnostic) =>
    mapDiagnosticOffsets(
      diagnostic,
      byteToUtf16,
      physicalSource === null
        ? null
        : {
            ...physicalSource,
            virtualLength: sourceText.length,
            virtualByteLength: utf8Length(sourceText),
          },
    ),
  );
  byRule.set(ruleName, { sourceText, scanSource, filename, diagnostics });
  return diagnostics;
}

function reportDiagnostic(context, diagnostic) {
  const descriptor = {
    messageId: diagnostic.messageId,
  };
  if (diagnostic.reportRange) {
    descriptor.node = { range: diagnostic.reportRange };
  } else {
    descriptor.loc = {
      start: {
        line: diagnostic.loc.startLine,
        column: diagnostic.loc.startColumn,
      },
      end: {
        line: diagnostic.loc.endLine,
        column: diagnostic.loc.endColumn,
      },
    };
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

function mapDiagnosticOffsets(diagnostic, byteToUtf16, virtualSource) {
  const mapped = {
    ...diagnostic,
    start: byteToUtf16(diagnostic.start),
    end: byteToUtf16(diagnostic.end),
  };
  if (diagnostic.fix) {
    mapped.fix = {
      start: byteToUtf16(diagnostic.fix.start),
      end: byteToUtf16(diagnostic.fix.end),
      replacement: diagnostic.fix.replacement,
    };
  }
  if (virtualSource === null) {
    return mapped;
  }
  mapped.reportRange = [
    diagnostic.start > virtualSource.frontmatterByteOffset + virtualSource.virtualByteLength
      ? diagnostic.start - virtualSource.frontmatterByteOffset
      : mapped.start - virtualSource.frontmatterOffset,
    diagnostic.end > virtualSource.frontmatterByteOffset + virtualSource.virtualByteLength
      ? diagnostic.end - virtualSource.frontmatterByteOffset
      : mapped.end - virtualSource.frontmatterOffset,
  ];
  if (mapped.fix) {
    const start = mapped.fix.start - virtualSource.frontmatterOffset;
    const end = mapped.fix.end - virtualSource.frontmatterOffset;
    mapped.fix =
      start >= 0 && end <= virtualSource.virtualLength ? { ...mapped.fix, start, end } : undefined;
  }
  return mapped;
}

function readPhysicalSource(context, filename, virtualSourceText) {
  const physicalFilename =
    typeof context.physicalFilename === 'string'
      ? context.physicalFilename
      : typeof context.getPhysicalFilename === 'function'
        ? context.getPhysicalFilename()
        : null;
  if (!physicalFilename || !physicalFilename.toLowerCase().endsWith('.astro')) {
    return null;
  }
  try {
    const sourceText = readFileSync(physicalFilename, 'utf8');
    if (sourceText === virtualSourceText) {
      return null;
    }
    return {
      sourceText,
      ...frontmatterContentOffsets(sourceText),
    };
  } catch {
    return null;
  }
}

function frontmatterContentOffsets(sourceText) {
  const match = /^(?:\uFEFF)?---[ \t]*(?:\r\n|[\n\r\u2028\u2029])/.exec(sourceText);
  const opening = match?.[0] ?? '';
  return {
    frontmatterOffset: opening.length,
    frontmatterByteOffset: utf8Length(opening),
  };
}

function looksLikeAstroTemplate(sourceText) {
  return /<(?:\/?[A-Za-z]|!|>)/.test(sourceText);
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

function utf8Length(sourceText) {
  let length = 0;
  for (const char of sourceText) {
    length += utf8ByteLength(char.codePointAt(0));
  }
  return length;
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
