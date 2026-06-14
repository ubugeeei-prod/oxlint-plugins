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

module.exports = { parse, parseForESLint };
