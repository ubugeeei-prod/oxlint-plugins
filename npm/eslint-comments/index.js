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

function locForContext(context, loc) {
  if (!context.sourceCode?.isESTree) {
    return loc;
  }

  return {
    start: {
      line: loc.start.line,
      column: Math.max(0, loc.start.column),
    },
    end: {
      line: loc.end.line,
      column: Math.max(0, loc.end.column),
    },
  };
}

// Forward the diagnostics from a Rust scan to Oxlint. Diagnostic locations are
// kept verbatim in the upstream replay harness. Oxlint itself rejects the
// upstream `column: -1` "whole line" sentinel, so clamp it only for Oxlint's
// ESTree-compatible runtime.
function reportDiagnostics(context, diagnostics) {
  for (const diagnostic of diagnostics) {
    const loc = locForContext(context, {
      start: { line: diagnostic.loc.startLine, column: diagnostic.loc.startColumn },
      end: { line: diagnostic.loc.endLine, column: diagnostic.loc.endColumn },
    });
    const descriptor = {
      messageId: diagnostic.messageId,
      loc,
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

const DIRECTIVE_KINDS = [
  'eslint',
  'eslint-disable',
  'eslint-disable-line',
  'eslint-disable-next-line',
  'eslint-enable',
  'eslint-env',
  'exported',
  'global',
  'globals',
];

const noUse = commentScanRule(
  {
    type: 'suggestion',
    docs: {
      description: 'disallow ESLint directive-comments',
      recommended: false,
      url: `${DOCS_BASE}#no-use`,
    },
    fixable: null,
    schema: [
      {
        type: 'object',
        properties: {
          allow: {
            type: 'array',
            items: { enum: DIRECTIVE_KINDS },
            additionalItems: false,
            uniqueItems: true,
          },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      disallow: 'Unexpected ESLint directive comment.',
    },
  },
  (comments, context) => {
    const allow = (context.options[0] && context.options[0].allow) || [];
    return native.scanNoUse(comments, allow);
  },
);

const requireDescription = commentScanRule(
  {
    type: 'suggestion',
    docs: {
      description: 'require include descriptions in ESLint directive-comments',
      recommended: false,
      url: `${DOCS_BASE}#require-description`,
    },
    fixable: null,
    schema: [
      {
        type: 'object',
        properties: {
          ignore: {
            type: 'array',
            items: { enum: DIRECTIVE_KINDS },
            additionalItems: false,
            uniqueItems: true,
          },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      missingDescription:
        'Unexpected undescribed directive comment. Include descriptions to explain why the comment is necessary.',
    },
  },
  (comments, context) => {
    const ignore = (context.options[0] && context.options[0].ignore) || [];
    return native.scanRequireDescription(comments, ignore);
  },
);

function firstTokenStart(context) {
  const tokens = context.sourceCode.ast && context.sourceCode.ast.tokens;
  if (!tokens || tokens.length === 0) {
    return null;
  }
  const first = tokens[0];
  return { line: first.loc.start.line, column: first.loc.start.column };
}

const disableEnablePair = commentScanRule(
  {
    type: 'suggestion',
    docs: {
      description: 'require a `eslint-enable` comment for every `eslint-disable` comment',
      recommended: true,
      url: `${DOCS_BASE}#disable-enable-pair`,
    },
    fixable: null,
    schema: [
      {
        type: 'object',
        properties: {
          allowWholeFile: { type: 'boolean' },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      missingPair: "Requires 'eslint-enable' directive.",
      missingRulePair: "Requires 'eslint-enable' directive for '{{ruleId}}'.",
    },
  },
  (comments, context) => {
    const allowWholeFile = !!(context.options[0] && context.options[0].allowWholeFile);
    return native.scanDisableEnablePair(comments, allowWholeFile, firstTokenStart(context));
  },
);

const noAggregatingEnable = commentScanRule(
  {
    type: 'suggestion',
    docs: {
      description: 'disallow a `eslint-enable` comment for multiple `eslint-disable` comments',
      recommended: true,
      url: `${DOCS_BASE}#no-aggregating-enable`,
    },
    fixable: null,
    schema: [],
    messages: {
      aggregatingEnable:
        'This `eslint-enable` comment affects {{count}} `eslint-disable` comments. An `eslint-enable` comment should be for an `eslint-disable` comment.',
    },
  },
  (comments) => native.scanNoAggregatingEnable(comments),
);

const noDuplicateDisable = commentScanRule(
  {
    type: 'problem',
    docs: {
      description: 'disallow duplicate `eslint-disable` comments',
      recommended: true,
      url: `${DOCS_BASE}#no-duplicate-disable`,
    },
    fixable: null,
    schema: [],
    messages: {
      duplicate: 'ESLint rules have been disabled already.',
      duplicateRule: "'{{ruleId}}' rule has been disabled already.",
    },
  },
  (comments) => native.scanNoDuplicateDisable(comments),
);

const noUnusedEnable = commentScanRule(
  {
    type: 'problem',
    docs: {
      description: 'disallow unused `eslint-enable` comments',
      recommended: true,
      url: `${DOCS_BASE}#no-unused-enable`,
    },
    fixable: null,
    schema: [],
    messages: {
      unused: 'ESLint rules are re-enabled but those have not been disabled.',
      unusedRule: "'{{ruleId}}' rule is re-enabled but it has not been disabled.",
    },
  },
  (comments) => native.scanNoUnusedEnable(comments),
);

const noRestrictedDisable = commentScanRule(
  {
    type: 'suggestion',
    docs: {
      description: 'disallow `eslint-disable` comments about specific rules',
      recommended: false,
      url: `${DOCS_BASE}#no-restricted-disable`,
    },
    fixable: null,
    schema: {
      type: 'array',
      items: { type: 'string' },
      uniqueItems: true,
    },
    messages: {
      disallow: "Disabling '{{ruleId}}' is not allowed.",
    },
  },
  (comments, context) => native.scanNoRestrictedDisable(comments, context.options || []),
);

// `no-unused-disable` needs the file's lint problems, which only exist at run
// time via `sourceCode.getDisableDirectives()`. It is an approximation of
// upstream's deprecated Linter-patch behavior and is skipped when the runtime
// does not expose disable directives.
const noUnusedDisable = {
  meta: {
    type: 'problem',
    docs: {
      description: 'disallow unused `eslint-disable` comments',
      recommended: false,
      url: `${DOCS_BASE}#no-unused-disable`,
    },
    deprecated: true,
    fixable: null,
    schema: [],
    messages: {
      unused: 'Unused eslint-disable directive (no problems were reported).',
      unusedRule: "Unused eslint-disable directive (no problems were reported from '{{ruleId}}').",
    },
  },
  createOnce(context) {
    return {
      Program() {
        const sourceCode = context.sourceCode;
        if (typeof sourceCode.getDisableDirectives !== 'function') {
          return;
        }

        const comments = collectComments(sourceCode);
        if (comments.length === 0) {
          return;
        }

        const { problems } = sourceCode.getDisableDirectives();
        const problemInputs = (problems || []).map((problem) => ({
          ruleId: problem.ruleId == null ? null : problem.ruleId,
          line: problem.loc.start.line,
          column: problem.loc.start.column,
        }));

        reportDiagnostics(context, native.scanNoUnusedDisable(comments, problemInputs));
      },
    };
  },
};

const rules = {
  'disable-enable-pair': disableEnablePair,
  'no-aggregating-enable': noAggregatingEnable,
  'no-duplicate-disable': noDuplicateDisable,
  'no-restricted-disable': noRestrictedDisable,
  'no-unlimited-disable': noUnlimitedDisable,
  'no-unused-disable': noUnusedDisable,
  'no-unused-enable': noUnusedEnable,
  'no-use': noUse,
  'require-description': requireDescription,
};

// Mirror of upstream's `recommended` config. Upstream recommends exactly these
// five rules; no-restricted-disable, no-unused-disable (deprecated), no-use, and
// require-description are intentionally excluded from upstream's recommended set.
const recommendedRuleNames = [
  'disable-enable-pair',
  'no-aggregating-enable',
  'no-duplicate-disable',
  'no-unlimited-disable',
  'no-unused-enable',
];

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
