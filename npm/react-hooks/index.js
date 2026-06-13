'use strict';

// Oxlint plugin port of eslint-plugin-react-hooks (MIT).
// The JavaScript layer is only an Oxlint/NAPI adapter; the rules-of-hooks
// classification runs in Rust through Oxc.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedReactHooksRuleNames, scanReactHooks } = require('./api.js');

const PLUGIN_NAME = 'react-hooks';
const DOCS_BASE = 'https://react.dev/reference/eslint-plugin-react-hooks/lints';
const diagnosticsCache = new WeakMap();

const messages = {
  'rules-of-hooks': {
    async: 'React Hook "{{hook}}" cannot be called in an async function.',
    loop: 'React Hook "{{hook}}" may be executed more than once. Possibly because it is called in a loop. React Hooks must be called in the exact same order in every component render.',
    conditional:
      'React Hook "{{hook}}" is called conditionally. React Hooks must be called in the exact same order in every component render.',
    class:
      'React Hook "{{hook}}" cannot be called in a class component. React Hooks must be called in a React function component or a custom React Hook function.',
    invalidFunction:
      'React Hook "{{hook}}" is called in function "{{functionName}}" that is neither a React function component nor a custom React Hook function. React component names must start with an uppercase letter. React Hook names must start with the word "use".',
    topLevel:
      'React Hook "{{hook}}" cannot be called at the top level. React Hooks must be called in a React function component or a custom React Hook function.',
    callback:
      'React Hook "{{hook}}" cannot be called inside a callback. React Hooks must be called in a React function component or a custom React Hook function.',
    tryCatch: 'React Hook "{{hook}}" cannot be called in a try/catch block.',
  },
};

const ruleDescriptions = {
  'rules-of-hooks': 'enforces the Rules of Hooks',
};

const implementedRuleNames = Object.freeze(implementedReactHooksRuleNames());

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createReactHooksRule(ruleName)]),
  ),
);

const recommendedRules = Object.freeze({
  [`${PLUGIN_NAME}/rules-of-hooks`]: 'error',
});

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  configs: {
    recommended: {
      name: `${PLUGIN_NAME}/recommended`,
      plugins: [PLUGIN_NAME],
      rules: recommendedRules,
    },
    'recommended-latest': {
      name: `${PLUGIN_NAME}/recommended-latest`,
      plugins: [PLUGIN_NAME],
      rules: recommendedRules,
    },
    all: {
      name: `${PLUGIN_NAME}/all`,
      plugins: [PLUGIN_NAME],
      rules: recommendedRules,
    },
  },
});

plugin.configs.flat = {
  recommended: {
    name: `${PLUGIN_NAME}/flat/recommended`,
    plugins: {
      [PLUGIN_NAME]: plugin,
    },
    rules: recommendedRules,
  },
  'recommended-latest': {
    name: `${PLUGIN_NAME}/flat/recommended-latest`,
    plugins: {
      [PLUGIN_NAME]: plugin,
    },
    rules: recommendedRules,
  },
};

plugin.implementedReactHooksRuleNames = implementedRuleNames;
plugin.scanReactHooks = scanReactHooks;

function createReactHooksRule(ruleName) {
  return {
    meta: {
      type: 'problem',
      docs: {
        description: ruleDescriptions[ruleName],
        recommended: true,
        url: `${DOCS_BASE}/${ruleName}`,
      },
      messages: messages[ruleName],
      schema: [],
    },
    createOnce(context) {
      return {
        Program() {
          for (const diagnostic of diagnosticsForContext(context)) {
            if (diagnostic.ruleName !== ruleName) {
              continue;
            }
            context.report({
              messageId: diagnostic.messageId,
              data: compactData(diagnostic.data),
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
        },
      };
    },
  };
}

function diagnosticsForContext(context) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.jsx';
  const cached = diagnosticsCache.get(sourceCode);

  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const diagnostics = scanReactHooks(sourceText, filename);
  diagnosticsCache.set(sourceCode, { sourceText, filename, diagnostics });
  return diagnostics;
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

function compactData(data) {
  const out = {};
  for (const [key, value] of Object.entries(data || {})) {
    if (value != null) {
      out[key] = value;
    }
  }
  return out;
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedReactHooksRuleNames = implementedRuleNames;
module.exports.scanReactHooks = scanReactHooks;
