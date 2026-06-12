'use strict';

const native = require('@oxlint-plugins/core').security;

function scanSecurity(sourceText, filename = 'file.js') {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanSecurity(sourceText, filename);
}

module.exports = {
  implementedSecurityRuleNames: native.implementedSecurityRuleNames,
  scanSecurity,
};
module.exports.default = module.exports;
