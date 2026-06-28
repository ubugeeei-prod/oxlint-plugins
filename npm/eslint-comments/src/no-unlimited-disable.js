'use strict';

// Port of eslint-plugin-eslint-comments `no-unlimited-disable`.
//
// Behavior is a clean reimplementation of the upstream all-comments path
// (lib/internal/get-all-directive-comments.js + lib/internal/utils.js), parsing
// each comment's text directly so the result is parser-independent. Parity with the
// upstream rule is enforced by the captured corpus under
// tools/parity/corpora/eslint-plugin-eslint-comments/no-unlimited-disable.json.

const DIRECTIVE_PATTERN =
  /^(eslint(?:-env|-enable|-disable(?:(?:-next)?-line)?)?|exported|globals?)(?:\s|$)/u;
const LINE_COMMENT_PATTERN = /^eslint-disable-(next-)?line$/u;

/** Split off a `-- description` trailer, matching upstream `divideDirectiveComment`. */
function divideDirectiveComment(value) {
  const divided = value.split(/\s-{2,}\s/u);
  return { text: divided[0].trim() };
}

/** Parse a comment's text into `{ kind, value }`, or null if not a directive. */
function parseDirectiveText(textToParse) {
  const { text } = divideDirectiveComment(textToParse);
  const match = DIRECTIVE_PATTERN.exec(text);
  if (!match) return null;
  const kind = match[1];
  const value = text.slice(match.index + kind.length).trim();
  return { kind, value };
}

/** Apply the comment-type and single-line constraints, matching upstream `parseDirectiveComment`. */
function parseDirectiveComment(comment) {
  const parsed = parseDirectiveText(comment.value);
  if (!parsed) return null;

  const lineCommentSupported = LINE_COMMENT_PATTERN.test(parsed.kind);
  if (comment.type === 'Line' && !lineCommentSupported) return null;

  if (parsed.kind === 'eslint-disable-line' && comment.loc.start.line !== comment.loc.end.line) {
    // An `eslint-disable-line` comment must not span multiple lines.
    return null;
  }
  return parsed;
}

const DISABLE_KINDS = new Set([
  'eslint-disable',
  'eslint-disable-line',
  'eslint-disable-next-line',
]);

module.exports = {
  meta: {
    type: 'suggestion',
    docs: {
      description: 'disallow `eslint-disable` comments without rule names',
      recommended: true,
      url: 'https://eslint-community.github.io/eslint-plugin-eslint-comments/rules/no-unlimited-disable.html',
    },
    fixable: null,
    schema: [],
    messages: {
      unexpected: "Unexpected unlimited '{{kind}}' comment. Specify some rule names to disable.",
    },
  },

  create(context) {
    return {
      Program() {
        for (const comment of context.sourceCode.getAllComments()) {
          const parsed = parseDirectiveComment(comment);
          if (!parsed || !DISABLE_KINDS.has(parsed.kind)) continue;
          if (!parsed.value) {
            context.report({
              loc: comment.loc,
              messageId: 'unexpected',
              data: { kind: parsed.kind },
            });
          }
        }
      },
    };
  },
};
