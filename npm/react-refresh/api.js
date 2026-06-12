'use strict';

const native = require('@oxlint-plugins/core').reactRefresh;

function defaultHocs() {
  return native.defaultHocs();
}

function isReactComponentName(name) {
  if (typeof name !== 'string') {
    throw new TypeError('name must be a string.');
  }
  return native.isReactComponentName(name);
}

function shouldScanFilename(filename, checkJS = false) {
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  return native.shouldScanFilename(filename, !!checkJS);
}

function isConstantExportExpressionKind(kind) {
  if (typeof kind !== 'string') {
    throw new TypeError('kind must be a string.');
  }
  return native.isConstantExportExpressionKind(kind);
}

function scanOnlyExportComponents(sourceText, filename, options = {}) {
  if (typeof sourceText !== 'string') {
    throw new TypeError('sourceText must be a string.');
  }
  if (typeof filename !== 'string') {
    throw new TypeError('filename must be a string.');
  }
  const extraHocs = options.extraHOCs ?? options.extraHocs;
  const checkJs = options.checkJS ?? options.checkJs;
  return native.scanOnlyExportComponents(sourceText, filename, {
    extraHocs: Array.isArray(extraHocs) ? extraHocs : [],
    allowExportNames: Array.isArray(options.allowExportNames) ? options.allowExportNames : [],
    allowConstantExport: options.allowConstantExport === true,
    checkJs: checkJs === true,
  });
}

module.exports = {
  defaultHocs,
  isConstantExportExpressionKind,
  isReactComponentName,
  scanOnlyExportComponents,
  shouldScanFilename,
};
module.exports.default = module.exports;
