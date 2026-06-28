'use strict';

const { eslintCompatPlugin } = require('@oxlint/plugins');

const imports = require('./src/imports.js');
const exports_ = require('./src/exports.js');

const PLUGIN_NAME = 'simple-import-sort';

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules: {
    imports,
    exports: exports_,
  },
});

module.exports = plugin;
module.exports.default = plugin;
