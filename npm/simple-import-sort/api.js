'use strict';

const native = require('./native.js');

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
  return groups
    .filter(Array.isArray)
    .map((group) => group.filter((value) => typeof value === 'string' && value.length > 0))
    .filter((group) => group.length > 0);
}

module.exports = {
  implementedSimpleImportSortRuleNames: native.implementedSimpleImportSortRuleNames,
  scanSimpleImportSort,
};
module.exports.default = module.exports;
