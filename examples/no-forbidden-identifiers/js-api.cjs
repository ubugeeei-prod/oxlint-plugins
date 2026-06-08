'use strict';

const {
  isForbiddenIdentifierName,
  scanForbiddenIdentifiers,
} = require('../../npm/no-forbidden-identifiers/api.js');

const sourceText = `
const event = data.error;
function run(ctx) {
  return ctx;
}
`;

const matches = scanForbiddenIdentifiers(sourceText, { names: ['ctx'] });

console.log({
  matches,
  eventIsForbidden: isForbiddenIdentifierName('event'),
  ctxIsForbidden: isForbiddenIdentifierName('ctx', { names: ['ctx'] }),
});
