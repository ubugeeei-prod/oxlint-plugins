'use strict';

const native = require('./native.js');

function scanAstro(sourceText, filename = 'file.astro', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanAstro(sourceText, filename, {
    ruleNames: normalizeStringArray(options.ruleNames),
    frontmatterOnly: options.frontmatterOnly === true,
  });
}

function normalizeStringArray(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.filter((value) => typeof value === 'string' && value.length > 0);
}

module.exports = {
  implementedAstroRuleNames: native.implementedAstroRuleNames,
  scanAstro,
};
module.exports.default = module.exports;
