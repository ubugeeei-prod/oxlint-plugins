'use strict';

// Public entry for the shared native core. Re-exports the NAPI binding, whose
// functions are grouped per plugin namespace (e.g. `noForbiddenIdentifiers`).
// Plugin packages depend on this and read their own namespace.
module.exports = require('./native.js');
