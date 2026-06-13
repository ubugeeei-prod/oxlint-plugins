'use strict';

const native = require('./native.js');

function implementedFunctionalRuleNames() {
  return native.implementedFunctionalRuleNames();
}

function scanFunctional(sourceText, filename = 'file.ts', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanFunctional(sourceText, filename, normalizeOptions(options));
}

function normalizeOptions(options) {
  const raw = options && typeof options === 'object' ? options : {};
  return {
    ruleNames: normalizeRuleNames(raw.ruleNames),
    allowRestParameter: raw.allowRestParameter === true,
    allowArgumentsKeyword: raw.allowArgumentsKeyword === true,
    allowLetInForLoopInit: raw.allowLetInForLoopInit === true,
    allowThrowToRejectPromises: raw.allowThrowToRejectPromises === true,
    allowTryCatch: raw.allowTryCatch === true,
    allowTryFinally: raw.allowTryFinally === true,
    readonlyTypeMode:
      raw.readonlyTypeMode === 'keyword' || raw.readonlyTypeMode === 'generic'
        ? raw.readonlyTypeMode
        : undefined,
  };
}

function normalizeRuleNames(ruleNames) {
  if (!Array.isArray(ruleNames)) {
    return undefined;
  }
  return ruleNames.filter((ruleName) => typeof ruleName === 'string' && ruleName.length > 0);
}

module.exports = {
  implementedFunctionalRuleNames,
  scanFunctional,
};
module.exports.default = module.exports;
