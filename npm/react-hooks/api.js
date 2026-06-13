'use strict';

const native = require('./native.js');

function scanReactHooks(sourceText, filename = 'file.jsx') {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.scanReactHooks(sourceText, filename);
}

module.exports = {
  implementedReactHooksRuleNames: native.implementedReactHooksRuleNames,
  isHookName: native.isHookName,
  isReactComponentName: native.isReactComponentName,
  scanReactHooks,
};
module.exports.default = module.exports;
