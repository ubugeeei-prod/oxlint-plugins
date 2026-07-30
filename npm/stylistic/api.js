'use strict';

const native = require('./native.js');

function runNativeStylisticLint(sourceText, config) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }

  return native
    .runNativeStylisticLint(sourceText, normalizeRunConfig(config))
    .map(normalizeDiagnosticData);
}

function nativeStylisticRuleMetas() {
  return native.nativeStylisticRuleMetas();
}

function normalizeRunConfig(config) {
  if (!config || !Array.isArray(config.rules)) {
    return { rules: [] };
  }

  return {
    ...(typeof config.filename === 'string' ? { filename: config.filename } : {}),
    rules: config.rules
      .filter((rule) => rule && typeof rule.name === 'string')
      .map((rule) => ({
        name: rule.name,
        options: Array.isArray(rule.options) ? rule.options : (rule.options ?? []),
      })),
  };
}

function normalizeDiagnosticData(diagnostic) {
  if (diagnostic.ruleName !== 'jsx-indent-props' || !diagnostic.data) {
    return diagnostic;
  }
  return {
    ...diagnostic,
    data: {
      ...diagnostic.data,
      needed: Number(diagnostic.data.needed),
      gotten: Number(diagnostic.data.gotten),
    },
  };
}

module.exports = {
  nativeStylisticRuleMetas,
  runNativeStylisticLint,
};
module.exports.default = module.exports;
