'use strict';

// Oxlint plugin port of @eslint-community/eslint-plugin-eslint-comments (MIT).
// Rule behavior and diagnostic message templates follow the upstream plugin
// (https://github.com/eslint-community/eslint-plugin-eslint-comments, v4.7.2).
// Rule logic runs in Rust through NAPI-RS; this wrapper collects comments once
// per file and maps the returned diagnostics onto `context.report`.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const native = require('./native.js');

const PLUGIN_NAME = 'eslint-comments';
const DOCS_BASE = 'https://github.com/ubugeeei-prod/oxlint-plugins/tree/main/npm/eslint-comments';

function collectComments(sourceCode) {
  const comments = sourceCode.getAllComments();
  const result = [];

  for (const comment of comments) {
    if (comment.type !== 'Line' && comment.type !== 'Block') {
      continue;
    }

    result.push({
      kind: comment.type,
      value: comment.value,
      startLine: comment.loc.start.line,
      startColumn: comment.loc.start.column,
      endLine: comment.loc.end.line,
      endColumn: comment.loc.end.column,
    });
  }

  return result;
}

function buildData(data) {
  if (!data) {
    return undefined;
  }

  const out = {};
  if (data.kind != null) {
    out.kind = data.kind;
  }
  if (data.ruleId != null) {
    out.ruleId = data.ruleId;
  }
  if (data.count != null) {
    out.count = data.count;
  }

  return Object.keys(out).length > 0 ? out : undefined;
}

// Forward the diagnostics from a Rust scan to Oxlint. Diagnostic locations are
// kept verbatim (including the upstream `column: -1` "whole line" sentinel).
function reportDiagnostics(context, diagnostics) {
  for (const diagnostic of diagnostics) {
    const descriptor = {
      messageId: diagnostic.messageId,
      loc: {
        start: { line: diagnostic.loc.startLine, column: diagnostic.loc.startColumn },
        end: { line: diagnostic.loc.endLine, column: diagnostic.loc.endColumn },
      },
    };

    const data = buildData(diagnostic.data);
    if (data) {
      descriptor.data = data;
    }

    context.report(descriptor);
  }
}

// Build a rule whose entire work is a single per-file Rust scan over comments.
function commentScanRule(meta, scan) {
  return {
    meta,
    createOnce(context) {
      return {
        Program() {
          const comments = collectComments(context.sourceCode);
          if (comments.length === 0) {
            return;
          }

          reportDiagnostics(context, scan(comments, context));
        },
      };
    },
  };
}

const noUnlimitedDisable = commentScanRule(
  {
    type: 'suggestion',
    docs: {
      description: 'disallow `eslint-disable` comments without rule names',
      recommended: true,
      url: `${DOCS_BASE}#no-unlimited-disable`,
    },
    fixable: null,
    schema: [],
    messages: {
      unexpected: "Unexpected unlimited '{{kind}}' comment. Specify some rule names to disable.",
    },
  },
  (comments) => native.scanNoUnlimitedDisable(comments),
);

const rules = {
  'no-unlimited-disable': noUnlimitedDisable,
};

// Mirror of upstream's `recommended` config, limited to the rules ported so far.
const recommendedRuleNames = ['no-unlimited-disable'];

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  configs: {
    recommended: {
      plugins: [PLUGIN_NAME],
      rules: Object.fromEntries(
        recommendedRuleNames.map((name) => [`${PLUGIN_NAME}/${name}`, 'error']),
      ),
    },
  },
});

module.exports = plugin;
module.exports.default = plugin;
