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
    allowedHooks: stringList(options.allowedHooks),
    allowedPrefixes: stringList(options.allowedPrefixes),
    assertFunctionNames: stringList(options.assertFunctionNames),
    assertFunctionPatterns: regexList(options.assertFunctionPatterns),
    expectAliases: stringList(options.expectAliases),
    ignore: stringList(options.ignore),
    ignoreTopLevelDescribe: options.ignoreTopLevelDescribe === true,
    hookAliases: hookAliases(options.hookAliases),
    testAliases: stringList(options.testAliases),
    maxExpects: integerOption(options.maxExpects, 'maxExpects', 1),
    maxNestedDescribe: integerOption(options.maxNestedDescribe, 'maxNestedDescribe', 0),
    maxTopLevelDescribes: numberOption(options.maxTopLevelDescribes, 'maxTopLevelDescribes', 1),
    restrictedLocators: listRestrictions(options.noRestrictedLocators, 'type'),
    restrictedMatchers: matcherRestrictions(options.noRestrictedMatchers),
    restrictedRoles: listRestrictions(options.noRestrictedRoles, 'role'),
    validTitle: validTitleOptions(options.validTitle),
    validTestTags: validTestTagsOptions(options.validTestTags),
  });
  const byteToUtf16 = createByteToUtf16Mapper(sourceText);
  return diagnostics.map((diagnostic) => mapDiagnosticFix(diagnostic, byteToUtf16));
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

function regexList(values) {
  return stringList(values).map((value) => {
    new RegExp(value);
    return value;
  });
}

function hookAliases(aliases) {
  if (!aliases || typeof aliases !== 'object' || Array.isArray(aliases)) {
    return [];
  }
  return Object.entries(aliases).flatMap(([hookName, names]) =>
    stringList(names).map((name) => ({ name, hookName })),
  );
}

function integerOption(value, name, minimum) {
  if (value === undefined) {
    return undefined;
  }
  if (!Number.isInteger(value) || value < minimum) {
    throw new TypeError(`${name} must be an integer greater than or equal to ${minimum}.`);
  }
  return value;
}

function numberOption(value, name, minimum) {
  if (value === undefined) {
    return undefined;
  }
  if (typeof value !== 'number' || !Number.isFinite(value) || value < minimum) {
    throw new TypeError(`${name} must be a finite number greater than or equal to ${minimum}.`);
  }
  return value;
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

module.exports = {
  implementedPlaywrightRuleNames,
  scanPlaywright,
};
module.exports.default = module.exports;
