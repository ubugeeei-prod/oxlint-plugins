'use strict';

const native = require('@oxlint-plugins/core').mocha;

function implementedMochaRuleNames() {
  return native.implementedMochaRuleNames();
}

function scanMocha(sourceText, filename = 'file.js', options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }

  return native.scanMocha(sourceText, filename, normalizeOptions(options));
}

function normalizeOptions(options) {
  const raw = options && typeof options === 'object' ? options : {};
  return {
    consistentInterface: normalizeString(raw.consistentInterface),
    maxTopLevelSuitesLimit:
      typeof raw.maxTopLevelSuitesLimit === 'number' ? raw.maxTopLevelSuitesLimit : undefined,
    handleDoneIgnorePending: raw.handleDoneIgnorePending === true,
    noHooksAllowed: normalizeStringArray(raw.noHooksAllowed),
    noHooksForSingleCaseAllowed: normalizeStringArray(raw.noHooksForSingleCaseAllowed),
    noSynchronousAllowed: Array.isArray(raw.noSynchronousAllowed)
      ? normalizeStringArray(raw.noSynchronousAllowed)
      : undefined,
    noEmptyTitleMessage: normalizeString(raw.noEmptyTitleMessage),
    validSuiteTitlePattern: normalizeString(raw.validSuiteTitlePattern),
    validSuiteTitleMessage: normalizeString(raw.validSuiteTitleMessage),
    validTestTitlePattern: normalizeString(raw.validTestTitlePattern),
    validTestTitleMessage: normalizeString(raw.validTestTitleMessage),
    preferArrowAllowNamedFunctions: raw.preferArrowAllowNamedFunctions === true,
    preferArrowAllowUnboundThis: raw.preferArrowAllowUnboundThis !== false,
  };
}

function normalizeString(value) {
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

function normalizeStringArray(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.filter((value) => typeof value === 'string' && value.length > 0);
}

module.exports = {
  implementedMochaRuleNames,
  scanMocha,
};
module.exports.default = module.exports;
