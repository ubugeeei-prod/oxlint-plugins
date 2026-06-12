'use strict';

// Oxlint plugin port of eslint-plugin-react-refresh (MIT).
// The JavaScript layer is only an Oxlint/NAPI adapter; parsing and rule
// classification run in Rust through Oxc.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const native = require('./native.js');

const PLUGIN_NAME = 'react-refresh';
const RULE_NAME = 'only-export-components';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/react-refresh';

function normalizeOptions(raw) {
  const options = raw && typeof raw === 'object' ? raw : {};
  return {
    extraHocs: Array.isArray(options.extraHOCs)
      ? options.extraHOCs.filter((value) => typeof value === 'string' && value.length > 0)
      : [],
    allowExportNames: Array.isArray(options.allowExportNames)
      ? options.allowExportNames.filter((value) => typeof value === 'string')
      : [],
    allowConstantExport: options.allowConstantExport === true,
    checkJs: options.checkJS === true,
  };
}

function sourceTextForContext(context) {
  const sourceCode = context.sourceCode || {};
  if (typeof sourceCode.getText === 'function') {
    return sourceCode.getText();
  }
  if (typeof sourceCode.text === 'string') {
    return sourceCode.text;
  }
  return '';
}

function reportDiagnostics(context, diagnostics) {
  for (const diagnostic of diagnostics) {
    context.report({
      messageId: diagnostic.messageId,
      loc: {
        start: {
          line: diagnostic.loc.startLine,
          column: diagnostic.loc.startColumn,
        },
        end: {
          line: diagnostic.loc.endLine,
          column: diagnostic.loc.endColumn,
        },
      },
    });
  }
}

const onlyExportComponents = {
  meta: {
    type: 'problem',
    docs: {
      description: 'validate that Fast Refresh files only export components',
      recommended: true,
      url: `${DOCS_BASE}#only-export-components`,
    },
    schema: [
      {
        type: 'object',
        properties: {
          extraHOCs: { type: 'array', items: { type: 'string' } },
          allowExportNames: { type: 'array', items: { type: 'string' } },
          allowConstantExport: { type: 'boolean' },
          checkJS: { type: 'boolean' },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      exportAll: "This rule can't verify that `export *` only exports components.",
      namedExport:
        'Fast refresh only works when a file only exports components. Use a new file to share constants or functions between components.',
      anonymousExport: "Fast refresh can't handle anonymous components. Add a name to your export.",
      localComponents:
        'Fast refresh only works when a file only exports components. Move your component(s) to a separate file. If all exports are HOCs, add them to the `extraHOCs` option.',
      noExport:
        'Fast refresh only works when a file has exports. Move your component(s) to a separate file.',
      reactContext:
        'Fast refresh only works when a file only exports components. Move your React context(s) to a separate file.',
    },
  },
  createOnce(context) {
    return {
      Program() {
        const diagnostics = native.scanOnlyExportComponents(
          sourceTextForContext(context),
          context.filename,
          normalizeOptions(context.options?.[0]),
        );
        reportDiagnostics(context, diagnostics);
      },
    };
  },
};

function buildConfig(name, baseOptions) {
  return (options = {}) => ({
    name: `${PLUGIN_NAME}/${name}`,
    plugins: [PLUGIN_NAME],
    rules: {
      [`${PLUGIN_NAME}/${RULE_NAME}`]: ['error', { ...baseOptions, ...options }],
    },
  });
}

const configs = {
  recommended: buildConfig('recommended', {}),
  vite: buildConfig('vite', { allowConstantExport: true }),
  next: buildConfig('next', {
    allowExportNames: [
      'experimental_ppr',
      'dynamic',
      'dynamicParams',
      'revalidate',
      'fetchCache',
      'runtime',
      'preferredRegion',
      'maxDuration',
      'metadata',
      'generateMetadata',
      'viewport',
      'generateViewport',
      'generateImageMetadata',
      'generateSitemaps',
      'generateStaticParams',
    ],
  }),
};

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules: {
    [RULE_NAME]: onlyExportComponents,
  },
  configs: {
    recommended: configs.recommended(),
    vite: configs.vite(),
    next: configs.next(),
  },
});

plugin.reactRefresh = { plugin, configs };

module.exports = plugin;
module.exports.default = plugin;
module.exports.reactRefresh = plugin.reactRefresh;
