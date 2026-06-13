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
    allowInFunctions: raw.allowInFunctions === true,
    allowThrowToRejectPromises: raw.allowThrowToRejectPromises === true,
    allowTryCatch: raw.allowTryCatch === true,
    allowTryFinally: raw.allowTryFinally === true,
    readonlyTypeMode:
      raw.readonlyTypeMode === 'keyword' || raw.readonlyTypeMode === 'generic'
        ? raw.readonlyTypeMode
        : undefined,
    ignoreIfReadonlyWrapped: raw.ignoreIfReadonlyWrapped === true,
    ignoreIdentifierPattern: normalizeStringList(raw.ignoreIdentifierPattern),
    ignoreCodePattern: normalizeStringList(raw.ignoreCodePattern),
    enforceParameterCount:
      typeof raw.enforceParameterCount === 'string' ? raw.enforceParameterCount : undefined,
    enforceCountIgnoreIife:
      typeof raw.enforceCountIgnoreIife === 'boolean' ? raw.enforceCountIgnoreIife : undefined,
    enforceCountIgnoreGettersSetters:
      typeof raw.enforceCountIgnoreGettersSetters === 'boolean'
        ? raw.enforceCountIgnoreGettersSetters
        : undefined,
    enforceCountIgnoreLambda:
      typeof raw.enforceCountIgnoreLambda === 'boolean' ? raw.enforceCountIgnoreLambda : undefined,
    ignorePrefixSelectorNames: normalizeStringList(raw.ignorePrefixSelectorNames),
    checkInterfaces: typeof raw.checkInterfaces === 'boolean' ? raw.checkInterfaces : undefined,
    checkTypeLiterals:
      typeof raw.checkTypeLiterals === 'boolean' ? raw.checkTypeLiterals : undefined,
    allowReturningBranches: raw.allowReturningBranches === true,
  };
}

function normalizeStringList(value) {
  if (typeof value === 'string') {
    return [value];
  }
  if (Array.isArray(value)) {
    return value.filter((item) => typeof item === 'string');
  }
  return undefined;
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
