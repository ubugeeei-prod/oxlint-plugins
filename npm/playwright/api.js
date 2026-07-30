'use strict';

const native = require('./native.js');

function implementedPlaywrightRuleNames() {
  return native.implementedPlaywrightRuleNames();
}

function scanPlaywright(sourceText, filename = 'file.spec.ts', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  if (!options || typeof options !== 'object' || Array.isArray(options)) {
    throw new TypeError('options must be an object.');
  }

  const diagnostics = native.scanPlaywright(sourceText, filename, {
    expectAliases: stringList(options.expectAliases),
    testAliases: stringList(options.testAliases),
    restrictedLocators: listRestrictions(options.noRestrictedLocators, 'type'),
    restrictedMatchers: matcherRestrictions(options.noRestrictedMatchers),
    restrictedRoles: listRestrictions(options.noRestrictedRoles, 'role'),
    validTitle: validTitleOptions(options.validTitle),
    validTestTags: validTestTagsOptions(options.validTestTags),
  });
  const byteToUtf16 = createByteToUtf16Mapper(sourceText);
  return diagnostics.map((diagnostic) => {
    if (!diagnostic.fix) {
      return diagnostic;
    }
    return {
      ...diagnostic,
      fix: {
        ...diagnostic.fix,
        start: byteToUtf16(diagnostic.fix.start),
        end: byteToUtf16(diagnostic.fix.end),
      },
    };
  });
}

function validTitleOptions(value) {
  const options = objectOrEmpty(value);
  return {
    disallowedWords: stringList(options.disallowedWords),
    ignoreSpaces: booleanOr(options.ignoreSpaces, false),
    ignoreTypeOfDescribeName: booleanOr(options.ignoreTypeOfDescribeName, false),
    ignoreTypeOfStepName: booleanOr(options.ignoreTypeOfStepName, true),
    ignoreTypeOfTestName: booleanOr(options.ignoreTypeOfTestName, false),
    mustMatch: titlePatternOptions(options.mustMatch),
    mustNotMatch: titlePatternOptions(options.mustNotMatch),
  };
}

function titlePatternOptions(value) {
  if (typeof value === 'string' || Array.isArray(value)) {
    const pattern = titlePattern(value);
    return pattern ? { describe: pattern, step: pattern, test: pattern } : {};
  }
  const groups = objectOrEmpty(value);
  return Object.fromEntries(
    ['describe', 'step', 'test'].flatMap((group) => {
      const pattern = titlePattern(groups[group]);
      return pattern ? [[group, pattern]] : [];
    }),
  );
}

function titlePattern(value) {
  const tuple = Array.isArray(value) ? value : [value];
  if (typeof tuple[0] !== 'string') {
    return null;
  }
  // Match upstream's eager `new RegExp(pattern, 'u')` validation and error.
  new RegExp(tuple[0], 'u');
  return {
    source: tuple[0],
    ...(typeof tuple[1] === 'string' ? { message: tuple[1] } : {}),
  };
}

function validTestTagsOptions(value) {
  const options = objectOrEmpty(value);
  const allowedTags = tagPatterns(options.allowedTags);
  const disallowedTags = tagPatterns(options.disallowedTags);
  if (allowedTags.length > 0 && disallowedTags.length > 0) {
    throw new Error('The allowedTags and disallowedTags options cannot be used together');
  }
  return { allowedTags, disallowedTags };
}

function tagPatterns(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.flatMap((value) => {
    if (typeof value === 'string') {
      if (!value.startsWith('@')) {
        throw new Error(`Invalid tag "${value}" in configuration: tags must start with @`);
      }
      return [{ source: value, flags: '', isRegex: false }];
    }
    if (value instanceof RegExp) {
      return [{ source: value.source, flags: value.flags, isRegex: true }];
    }
    if (value && typeof value === 'object' && typeof value.source === 'string') {
      return [
        {
          source: value.source,
          flags: typeof value.flags === 'string' ? value.flags : '',
          isRegex: true,
        },
      ];
    }
    return [];
  });
}

function listRestrictions(values, key) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.flatMap((value) => {
    if (typeof value === 'string') {
      return [{ value }];
    }
    if (!value || typeof value !== 'object' || typeof value[key] !== 'string') {
      return [];
    }
    return [
      {
        value: value[key],
        ...(typeof value.message === 'string' ? { message: value.message } : {}),
      },
    ];
  });
}

function matcherRestrictions(values) {
  if (!values || typeof values !== 'object' || Array.isArray(values)) {
    return [];
  }
  return Object.entries(values).flatMap(([value, message]) =>
    message === null || typeof message === 'string'
      ? [{ value, ...(message === null ? {} : { message }) }]
      : [],
  );
}

function stringList(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.filter((value) => typeof value === 'string');
}

function objectOrEmpty(value) {
  return value && typeof value === 'object' && !Array.isArray(value) ? value : {};
}

function booleanOr(value, fallback) {
  return typeof value === 'boolean' ? value : fallback;
}

function createByteToUtf16Mapper(sourceText) {
  const nonAsciiSpans = [];
  let byteOffset = 0;
  let utf16Offset = 0;

  while (utf16Offset < sourceText.length) {
    const codePoint = sourceText.codePointAt(utf16Offset);
    if (codePoint === undefined) break;
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
  return (offset) => {
    const clampedByteOffset = clampOffset(offset, byteOffset);
    let low = 0;
    let high = nonAsciiSpans.length;
    while (low < high) {
      const mid = Math.floor((low + high) / 2);
      if (nonAsciiSpans[mid].byteEnd <= clampedByteOffset) low = mid + 1;
      else high = mid;
    }
    const nextSpan = nonAsciiSpans[low];
    if (
      nextSpan &&
      clampedByteOffset >= nextSpan.byteStart &&
      clampedByteOffset < nextSpan.byteEnd
    ) {
      return nextSpan.utf16Start;
    }
    const delta = nonAsciiSpans[low - 1]?.deltaAfter ?? 0;
    return clampOffset(clampedByteOffset - delta, sourceText.length);
  };
}

function utf8ByteLength(codePoint) {
  if (codePoint <= 0x7f) return 1;
  if (codePoint <= 0x7ff) return 2;
  if (codePoint <= 0xffff) return 3;
  return 4;
}

function clampOffset(offset, max) {
  if (!Number.isFinite(offset) || offset <= 0) return 0;
  if (offset >= max) return max;
  return Math.trunc(offset);
}

module.exports = {
  implementedPlaywrightRuleNames,
  scanPlaywright,
};
module.exports.default = module.exports;
