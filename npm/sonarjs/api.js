'use strict';

try {
  const native = require('./native.js');
  module.exports = {
    ...native,
    implementedSonarjsRuleNames: native.implementedSonarjsRuleNames,
    scanSonarjs: native.scanSonarjs,
  };
} catch (error) {
  if (error && error.code === 'MODULE_NOT_FOUND') {
    throw new Error(
      'Native sonarjs binding is missing. Run `pnpm --filter @oxlint-plugins/oxlint-plugin-sonarjs build`.',
    );
  }
  throw error;
}
