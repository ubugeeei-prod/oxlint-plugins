'use strict';

// Rust-backed port of postgresql-eslint-parser (MIT). The custom-parser entry
// point ESLint loads: it exposes `parseForESLint` (and `parse`). All SQL parsing
// runs in Rust (libpg_query, PostgreSQL 17); this layer is only a NAPI adapter.

module.exports = require('./api.js');
