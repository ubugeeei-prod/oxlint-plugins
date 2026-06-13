'use strict';

try {
  const native = require('./native.js');
  module.exports = {
    ...native,
    implementedE18eRuleNames: native.implementedE18ERuleNames,
    scanE18e: native.scanE18E,
  };
} catch (error) {
  if (error && error.code === 'MODULE_NOT_FOUND') {
    throw new Error(
      'Native e18e binding is missing. Run `pnpm --filter @oxlint-plugins/oxlint-plugin-e18e build`.',
    );
  }
  throw error;
}
