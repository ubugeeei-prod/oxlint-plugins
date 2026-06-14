'use strict';

// ESLint processor that lints PL function bodies embedded in SQL, ported
// verbatim from upstream `src/processor.ts`. `preprocess` extracts each PL body
// into a virtual file (keyed by the configured LANGUAGE → extension map);
// `postprocess` translates the lint messages' line/column (and dollar-quote fix
// ranges) back into the original SQL's coordinate system.

const { extractEmbeddedCode, parseForESLint } = require('./api.js');

const PROCESSOR_VERSION = '1';

// Convert a (line, column) pair reported in the virtual file's coordinate
// system back to the original SQL's coordinate system. ESLint reports both
// values 1-indexed; our parser exposes line 1-indexed but column 0-indexed, so
// when the message lands on the first line of the body we add columns
// (0-indexed + 1-indexed = 1-indexed). For lines past the first, columns reset
// to the start of the line and need no offset.
function translateLineColumn(body, line, column) {
  const sqlLine = body.loc.start.line + (line - 1);
  const sqlColumn = line === 1 ? body.loc.start.column + column : column;
  return { line: sqlLine, column: sqlColumn };
}

function translateMessage(message, body) {
  const translated = { ...message };

  if (typeof message.line === 'number' && typeof message.column === 'number') {
    const start = translateLineColumn(body, message.line, message.column);
    translated.line = start.line;
    translated.column = start.column;
  }

  if (typeof message.endLine === 'number' && typeof message.endColumn === 'number') {
    const end = translateLineColumn(body, message.endLine, message.endColumn);
    translated.endLine = end.line;
    translated.endColumn = end.column;
  }

  // Fix ranges are absolute character offsets in the file ESLint linted — i.e.
  // the virtual file. Translating them to the original SQL only works cleanly
  // for dollar-quoted bodies, where the virtual file's characters map 1:1 to a
  // contiguous slice of the SQL. Single-quoted bodies with `''` escapes would
  // need a sourceMap; drop fixes there rather than report wrong ranges.
  if (message.fix) {
    if (body.quoteStyle === 'dollar') {
      translated.fix = {
        range: [body.range[0] + message.fix.range[0], body.range[0] + message.fix.range[1]],
        text: message.fix.text,
      };
    } else {
      delete translated.fix;
    }
  }

  return translated;
}

/**
 * Create an ESLint processor that lints embedded PL function bodies.
 * @param {{ languages: Record<string, string>, unknown?: 'skip' | 'error' }} options
 *   `languages` maps a lower-cased LANGUAGE clause to a virtual-file extension
 *   (which MUST start with `.`); `unknown` controls bodies whose language is not
 *   mapped (`'skip'` drops them, `'error'` throws).
 */
function createPlProcessor(options) {
  const { languages, unknown = 'skip' } = options;
  // ESLint always calls postprocess right after preprocess for the same file,
  // so caching by filename is enough — even when several files are linted
  // concurrently they each have a distinct key.
  const blockCache = new Map();

  return {
    meta: {
      name: 'postgresql-eslint-parser/processor',
      version: PROCESSOR_VERSION,
    },
    supportsAutofix: true,
    preprocess(text, filename) {
      const { ast } = parseForESLint(text);
      const bodies = extractEmbeddedCode(ast);
      const blocks = [];

      for (const body of bodies) {
        const ext = languages[body.language];
        if (ext == null) {
          if (unknown === 'error') {
            throw new Error(
              `postgresql-eslint-parser/processor: no virtual-file extension configured for LANGUAGE "${body.language}"`,
            );
          }
          continue;
        }
        blocks.push({ body, ext });
      }

      blockCache.set(
        filename,
        blocks.map(({ body }) => ({ body })),
      );

      return blocks.map(({ body, ext }, index) => ({
        text: body.source,
        filename: `${index}${ext}`,
      }));
    },
    postprocess(messageLists, filename) {
      const blocks = blockCache.get(filename) ?? [];
      blockCache.delete(filename);

      const result = [];
      for (let i = 0; i < messageLists.length; i++) {
        const block = blocks[i];
        const messages = messageLists[i];
        if (!block || !messages) continue;
        for (const message of messages) {
          result.push(translateMessage(message, block.body));
        }
      }
      return result;
    },
  };
}

module.exports = { createPlProcessor };
