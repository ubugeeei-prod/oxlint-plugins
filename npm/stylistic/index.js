'use strict';

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { nativeStylisticRuleMetas, runNativeStylisticLint } = require('./api.js');

const PLUGIN_NAME = 'stylistic';
const SOURCE_URL = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/stylistic';

const stylisticMetas = nativeStylisticRuleMetas();
const stylisticMetasByName = new Map(stylisticMetas.map((meta) => [meta.name, meta]));
const implementedStylisticRuleNames = Object.freeze(stylisticMetas.map((meta) => meta.name));
const diagnosticsCache = new WeakMap();

const stylisticRules = Object.freeze(
  Object.fromEntries(
    implementedStylisticRuleNames.map((ruleName) => [ruleName, createStylisticRule(ruleName)]),
  ),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules: stylisticRules,
  configs: {
    recommended: {
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        implementedStylisticRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'error']),
      ),
    },
  },
});

plugin.corsaStylisticPlugin = plugin;
plugin.corsaStylisticRules = stylisticRules;
plugin.implementedStylisticRuleNames = implementedStylisticRuleNames;
plugin.nativeStylisticRuleMetas = nativeStylisticRuleMetas;
plugin.runNativeStylisticLint = runNativeStylisticLint;

function createStylisticRule(ruleName) {
  const meta = stylisticRuleMeta(ruleName);

  return {
    meta: {
      type: 'layout',
      docs: {
        description: meta.docsDescription,
        recommended: false,
        requiresTypeChecking: false,
        url: SOURCE_URL,
      },
      fixable: 'whitespace',
      hasSuggestions: meta.hasSuggestions,
      messages: meta.messages,
      schema: { type: 'array' },
    },
    createOnce(context) {
      return {
        Program(node) {
          reportStylisticDiagnostics(
            context,
            node,
            diagnosticsForRule(context, ruleName).filter(
              (diagnostic) => diagnostic.ruleName === ruleName,
            ),
          );
        },
      };
    },
  };
}

function diagnosticsForRule(context, ruleName) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const config = stylisticRunConfig(context, ruleName);
  const key = JSON.stringify(config);
  let sourceCache = diagnosticsCache.get(sourceCode);

  if (!sourceCache) {
    sourceCache = new Map();
    diagnosticsCache.set(sourceCode, sourceCache);
  }

  const cached = sourceCache.get(key);
  if (cached && cached.sourceText === sourceText) {
    return cached.diagnostics;
  }

  const diagnostics = mapNativeDiagnosticRanges(
    runNativeStylisticLint(sourceText, { rules: config }),
    sourceText,
  );
  sourceCache.set(key, { sourceText, diagnostics });
  return diagnostics;
}

function stylisticRunConfig(context, currentRuleName) {
  const settingsRules = context.settings?.corsaStylistic?.rules;
  const rules = new Map();

  if (settingsRules) {
    for (const [name, options] of Object.entries(settingsRules)) {
      assertKnownStylisticRuleName(name);
      rules.set(name, normalizeOptions(options));
    }
  }

  const currentOptions = currentRuleOptions(context);
  if (!rules.has(currentRuleName) || currentOptions.length > 0) {
    rules.set(currentRuleName, currentOptions);
  }

  return implementedStylisticRuleNames
    .filter((name) => rules.has(name))
    .map((name) => ({
      name,
      options: rules.get(name) ?? [],
    }));
}

function reportStylisticDiagnostics(context, program, diagnostics) {
  for (const diagnostic of diagnostics) {
    const descriptor = {
      node: rangeNode(program, diagnostic.range),
      messageId: diagnostic.messageId,
    };

    if (diagnostic.suggestions?.length) {
      descriptor.suggest = diagnostic.suggestions.map((suggestion) => ({
        messageId: suggestion.messageId,
        fix: (fixer) =>
          suggestion.fixes.map((fix) =>
            fixer.replaceTextRange(oxlintRange(fix.range), fix.replacementText),
          ),
      }));
    }

    context.report(descriptor);
  }
}

function rangeNode(program, range) {
  return {
    ...program,
    range: oxlintRange(range),
  };
}

function oxlintRange(range) {
  return [range.start, range.end];
}

function mapNativeDiagnosticRanges(diagnostics, sourceText) {
  const byteToUtf16 = createByteToUtf16Mapper(sourceText);
  return diagnostics.map((diagnostic) => ({
    ...diagnostic,
    range: mapNativeRange(diagnostic.range, byteToUtf16),
    ...(diagnostic.suggestions?.length
      ? {
          suggestions: diagnostic.suggestions.map((suggestion) =>
            mapNativeSuggestion(suggestion, byteToUtf16),
          ),
        }
      : {}),
  }));
}

function mapNativeSuggestion(suggestion, byteToUtf16) {
  return {
    ...suggestion,
    fixes: suggestion.fixes.map((fix) => mapNativeFix(fix, byteToUtf16)),
  };
}

function mapNativeFix(fix, byteToUtf16) {
  return {
    ...fix,
    range: mapNativeRange(fix.range, byteToUtf16),
  };
}

function mapNativeRange(range, byteToUtf16) {
  return {
    start: byteToUtf16(range.start),
    end: byteToUtf16(range.end),
  };
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
  const text = context.sourceCode?.text;
  if (typeof text === 'string') {
    return text;
  }
  return context.sourceCode.getText({ type: 'Program' });
}

function currentRuleOptions(context) {
  return normalizeOptions(context.options);
}

function normalizeOptions(options) {
  if (Array.isArray(options)) {
    return options;
  }
  if (options == null) {
    return [];
  }
  return [options];
}

function assertKnownStylisticRuleName(name) {
  if (!implementedStylisticRuleNames.includes(name)) {
    throw new Error(`unknown stylistic rule: ${name}`);
  }
}

function stylisticRuleMeta(ruleName) {
  const meta = stylisticMetasByName.get(ruleName);
  if (!meta) {
    throw new Error(`stylistic native Rust rule is not registered: ${ruleName}`);
  }
  return meta;
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.corsaStylisticPlugin = plugin;
module.exports.corsaStylisticRules = stylisticRules;
module.exports.implementedStylisticRuleNames = implementedStylisticRuleNames;
module.exports.nativeStylisticRuleMetas = nativeStylisticRuleMetas;
module.exports.runNativeStylisticLint = runNativeStylisticLint;
