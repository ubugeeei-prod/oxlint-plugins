'use strict';

const native = require('./native.js');

function scanUnocss(sourceText, filename = 'file.tsx', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }

  return native.scanUnocss(sourceText, filename, {
    unoFunctions: normalizeStringArray(options.unoFunctions),
    unoVariables: normalizeStringArray(options.unoVariables),
    blocklist: normalizeBlocklist(options.blocklist),
    classCompilePrefix:
      typeof options.classCompilePrefix === 'string' ? options.classCompilePrefix : undefined,
    classCompileEnableFix:
      typeof options.classCompileEnableFix === 'boolean'
        ? options.classCompileEnableFix
        : undefined,
  });
}

function normalizeStringArray(values) {
  if (!Array.isArray(values)) {
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
          reason: typeof value[1]?.message === 'string' ? value[1].message : undefined,
        };
      }
      if (value && typeof value.name === 'string') {
        return {
          name: value.name,
          reason: typeof value.reason === 'string' ? value.reason : undefined,
        };
      }
      return null;
    })
    .filter(Boolean);
}

module.exports = {
  implementedUnocssRuleNames: native.implementedUnocssRuleNames,
  scanUnocss,
};
module.exports.default = module.exports;
