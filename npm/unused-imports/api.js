'use strict';

const native = require('@oxlint-plugins/core').unusedImports;

function scanUnusedImports(sourceText, filename = 'file.js', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanUnusedImports(sourceText, filename, {
    ruleNames: normalizeRuleNames(options.ruleNames),
  });
}

function normalizeRuleNames(ruleNames) {
  if (!Array.isArray(ruleNames)) {
    return undefined;
  }
  return ruleNames.filter((ruleName) => typeof ruleName === 'string' && ruleName.length > 0);
}

module.exports = {
  implementedUnusedImportsRuleNames: native.implementedUnusedImportsRuleNames,
  scanUnusedImports,
};
module.exports.default = module.exports;
