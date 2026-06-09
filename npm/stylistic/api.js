'use strict';

const native = require('./native.js');

function runNativeStylisticLint(sourceText, config) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }

  return native.runNativeStylisticLint(sourceText, normalizeRunConfig(config));
}

function nativeStylisticRuleMetas() {
  return native.nativeStylisticRuleMetas();
}

function normalizeRunConfig(config) {
  if (!config || !Array.isArray(config.rules)) {
    return { rules: [] };
  }

  return {
    rules: config.rules
      .filter((rule) => rule && typeof rule.name === 'string')
      .map((rule) => ({
        name: rule.name,
        options: Array.isArray(rule.options) ? rule.options : (rule.options ?? []),
      })),
  };
}

module.exports = {
  nativeStylisticRuleMetas,
  runNativeStylisticLint,
};
module.exports.default = module.exports;
