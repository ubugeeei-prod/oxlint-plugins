'use strict';

const native = require('./native.js');

function implementedAngularEslintRuleNames() {
  return native.implementedAngularEslintRuleNames();
}

function scanAngularEslint(sourceText, filename = 'file.ts') {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }

  return native.scanAngularEslint(sourceText, filename);
}

module.exports = {
  implementedAngularEslintRuleNames,
  scanAngularEslint,
};
module.exports.default = module.exports;
