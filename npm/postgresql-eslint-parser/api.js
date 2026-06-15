'use strict';

// Thin adapter over the NAPI boundary. The native module returns the
// `parseForESLint` result as a JSON string; we parse it back into the object an
// ESLint custom parser must return: `{ ast, visitorKeys, scopeManager }`.

const native = require('./native.js');

/**
 * Parse PostgreSQL SQL into an ESLint-compatible AST.
 * @param {string} code SQL source text.
 * @returns {{ ast: object, visitorKeys: Record<string, string[]>, scopeManager: null }}
 */
function parseForESLint(code) {
  if (typeof code !== 'string') {
    throw new TypeError('code must be a string.');
  }
  // `parseForEslintJson` is napi-rs's camelCase of the Rust `parse_for_eslint_json`
  // (hence `Eslint`, not `ESLint`); it returns the result as a JSON string.
  return JSON.parse(native.parseForEslintJson(code));
}

/**
 * Parse PostgreSQL SQL and return only the AST (upstream's `parse` export).
 * @param {string} code SQL source text.
 * @returns {object} the `Program` AST node.
 */
function parse(code) {
  return parseForESLint(code).ast;
}

/**
 * Collect every `EmbeddedCode` node in a parsed program, in source order.
 * Mirrors upstream `extractEmbeddedCode`: the parser attaches an `embeddedCode`
 * node to each top-level `CreateFunctionStmt` that carries a PL body, so this
 * walks `program.body` and gathers them.
 * @param {object} program the `Program` AST node from {@link parseForESLint}.
 * @returns {object[]} the `EmbeddedCode` nodes (`{ type, language, source, quoteStyle, range, loc }`).
 */
function extractEmbeddedCode(program) {
  const result = [];
  const body = program && Array.isArray(program.body) ? program.body : [];
  for (const node of body) {
    const embedded = node && node.embeddedCode;
    if (embedded && embedded.type === 'EmbeddedCode') {
      result.push(embedded);
    }
  }
  return result;
}

module.exports = { extractEmbeddedCode, parse, parseForESLint };
