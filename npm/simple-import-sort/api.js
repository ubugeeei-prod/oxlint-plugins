'use strict';

const native = require('@oxlint-plugins/core').simpleImportSort;

function scanSimpleImportSort(sourceText, filename = 'file.js', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanSimpleImportSort(sourceText, filename, {
    importGroups: normalizeGroups(options.importGroups),
  });
}

function normalizeGroups(groups) {
  if (!Array.isArray(groups)) {
    return undefined;
  }
  // Preserve an explicitly-empty array as [] so the native layer can distinguish
  // "no groups option" (undefined) from "explicit empty groups" ([]).
  return groups
    .filter(Array.isArray)
    .map((group) => group.filter((value) => typeof value === 'string' && value.length > 0));
}

module.exports = {
  implementedSimpleImportSortRuleNames: native.implementedSimpleImportSortRuleNames,
  scanSimpleImportSort,
};
module.exports.default = module.exports;
