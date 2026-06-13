'use strict';

const native = require('./native.js');

function scanEslintJson(sourceText, options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  return native.scanEslintJson(sourceText, normalizeOptions(options));
}

function normalizeOptions(raw) {
  const options = raw && typeof raw === 'object' ? raw : {};
  return {
    ruleNames: normalizeRuleNames(options.ruleNames),
    normalizationForm: normalizeNormalizationForm(options.normalizationForm),
    sortDirection: normalizeSortDirection(options.sortDirection),
    sortCaseSensitive:
      typeof options.sortCaseSensitive === 'boolean' ? options.sortCaseSensitive : undefined,
    sortNatural: typeof options.sortNatural === 'boolean' ? options.sortNatural : undefined,
    sortMinKeys:
      Number.isInteger(options.sortMinKeys) && options.sortMinKeys >= 2
        ? options.sortMinKeys
        : undefined,
    sortAllowLineSeparatedGroups:
      typeof options.sortAllowLineSeparatedGroups === 'boolean'
        ? options.sortAllowLineSeparatedGroups
        : undefined,
  };
}

function normalizeRuleNames(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.filter((value) => typeof value === 'string' && value.length > 0);
}

function normalizeNormalizationForm(value) {
  return ['NFC', 'NFD', 'NFKC', 'NFKD'].includes(value) ? value : undefined;
}

function normalizeSortDirection(value) {
  return ['asc', 'desc', 'ascending', 'descending'].includes(value) ? value : undefined;
}

module.exports = {
  implementedEslintJsonRuleNames: native.implementedEslintJsonRuleNames,
  scanEslintJson,
};
module.exports.default = module.exports;
