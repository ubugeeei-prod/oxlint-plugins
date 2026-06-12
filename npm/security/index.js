'use strict';

// Oxlint plugin port of eslint-plugin-security (Apache-2.0).
// The JavaScript layer is only an Oxlint/NAPI adapter; parsing, import/require
// tracking, static-expression classification, and rule checks run in Rust.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedSecurityRuleNames, scanSecurity } = require('./api.js');

const PLUGIN_NAME = 'security';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/security';
const diagnosticsCache = new WeakMap();

const messages = {
  'detect-bidi-characters': {
    code: "Detected potential trojan source attack with unicode bidi introduced in this code: '{{text}}'.",
    comment:
      "Detected potential trojan source attack with unicode bidi introduced in this comment: '{{text}}'.",
  },
  'detect-buffer-noassert': {
    found: 'Found Buffer.{{method}} with noAssert flag set true',
  },
  'detect-child-process': {
    require: 'Found require("{{value}}")',
    execNonLiteral: 'Found child_process.exec() with non Literal first argument',
  },
  'detect-disable-mustache-escape': {
    found: 'Markup escaping disabled.',
  },
  'detect-eval-with-expression': {
    nonLiteral: 'eval with argument of type {{argumentType}}',
  },
  'detect-new-buffer': {
    found: 'Found new Buffer',
  },
  'detect-no-csrf-before-method-override': {
    found: 'express.csrf() middleware found before express.methodOverride()',
  },
  'detect-non-literal-fs-filename': {
    nonLiteral:
      'Found {{fnName}} from package "{{packageName}}" with non literal argument at index {{indices}}',
  },
  'detect-non-literal-regexp': {
    nonLiteral: 'Found non-literal argument to RegExp Constructor',
  },
  'detect-non-literal-require': {
    nonLiteral: 'Found non-literal argument in require',
  },
  'detect-object-injection': {
    variable: 'Variable Assigned to Object Injection Sink',
    functionCall: 'Function Call Object Injection Sink',
    generic: 'Generic Object Injection Sink',
  },
  'detect-possible-timing-attacks': {
    found: 'Potential timing attack, {{side}} side: true',
  },
  'detect-pseudoRandomBytes': {
    found: 'Found crypto.pseudoRandomBytes which does not produce cryptographically strong numbers',
  },
  'detect-unsafe-regex': {
    literal: 'Unsafe Regular Expression',
    newRegExp: 'Unsafe Regular Expression (new RegExp)',
  },
};

const ruleDescriptions = {
  'detect-bidi-characters':
    'Detects trojan source attacks that employ unicode bidi attacks to inject malicious code.',
  'detect-buffer-noassert': 'Detects calls to "buffer" with "noAssert" flag set.',
  'detect-child-process': 'Detects instances of "child_process" & non-literal "exec()" calls.',
  'detect-disable-mustache-escape':
    'Detects "object.escapeMarkup = false", which can be used with some template engines to disable escaping of HTML entities.',
  'detect-eval-with-expression':
    'Detects "eval(variable)" which can allow an attacker to run arbitrary code inside your process.',
  'detect-new-buffer':
    'Detects instances of new Buffer(argument) where argument is any non-literal value.',
  'detect-no-csrf-before-method-override':
    'Detects Express "csrf" middleware setup before "method-override" middleware.',
  'detect-non-literal-fs-filename':
    'Detects variable in filename argument of "fs" calls, which might allow an attacker to access anything on your system.',
  'detect-non-literal-regexp':
    'Detects "RegExp(variable)", which might allow an attacker to DOS your server with a long-running regular expression.',
  'detect-non-literal-require':
    'Detects "require(variable)", which might allow an attacker to load and run arbitrary code, or access arbitrary files on disk.',
  'detect-object-injection': 'Detects "variable[key]" as a left- or right-hand assignment operand.',
  'detect-possible-timing-attacks':
    'Detects insecure comparisons (`==`, `!=`, `!==` and `===`), which check input sequentially.',
  'detect-pseudoRandomBytes':
    'Detects if "pseudoRandomBytes()" is in use, which might not give you the randomness you need and expect.',
  'detect-unsafe-regex':
    'Detects potentially unsafe regular expressions, which may take a very long time to run, blocking the event loop.',
};

const implementedRuleNames = Object.freeze(implementedSecurityRuleNames());

const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createSecurityRule(ruleName)]),
  ),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 0])),
  configs: {
    recommended: {
      name: `${PLUGIN_NAME}/recommended`,
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'warn']),
      ),
    },
    'recommended-legacy': {
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        implementedRuleNames.map((ruleName) => [`${PLUGIN_NAME}/${ruleName}`, 'warn']),
      ),
    },
  },
});

plugin.implementedSecurityRuleNames = implementedRuleNames;
plugin.scanSecurity = scanSecurity;

function createSecurityRule(ruleName) {
  return {
    meta: {
      type: 'problem',
      docs: {
        description: ruleDescriptions[ruleName],
        category: 'Possible Security Vulnerability',
        recommended: true,
        url: `${DOCS_BASE}#${ruleName}`,
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
  const filename = typeof context.filename === 'string' ? context.filename : 'file.js';
  const cached = diagnosticsCache.get(sourceCode);

  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const diagnostics = scanSecurity(sourceText, filename);
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
module.exports.implementedSecurityRuleNames = implementedRuleNames;
module.exports.scanSecurity = scanSecurity;
