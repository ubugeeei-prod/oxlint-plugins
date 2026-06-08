import { describe, expect, it } from 'vitest';

import { scanForbiddenIdentifiers } from '../../npm/no-forbidden-identifiers/api.js';
import {
  applyTextEdits,
  createTextDocument,
  diagnosticForIdentifier,
  quickFixReplaceIdentifier,
} from './harness.mjs';

describe('LSP fixture harness', () => {
  it('snapshots diagnostics and quick fixes for a Rust-backed rule', () => {
    const document = createTextDocument('file:///fixture.ts', 'const event = data.error;\n');
    const diagnostics = scanForbiddenIdentifiers(document.text).map((name) =>
      diagnosticForIdentifier(
        document,
        'no-forbidden-identifiers/no-forbidden-identifiers',
        name,
        `Identifier '${name}' is reserved by this project policy.`,
      ),
    );
    const quickFix = quickFixReplaceIdentifier(document, diagnostics[0], 'reservedEvent');
    const updated = applyTextEdits(document.text, quickFix.edit.changes[document.uri]);

    expect({ diagnostics, quickFix, updated }).toMatchInlineSnapshot(`
      {
        "diagnostics": [
          {
            "code": "no-forbidden-identifiers/no-forbidden-identifiers",
            "data": {
              "identifier": "event",
              "ruleId": "no-forbidden-identifiers/no-forbidden-identifiers",
            },
            "message": "Identifier 'event' is reserved by this project policy.",
            "range": {
              "end": {
                "character": 11,
                "line": 0,
              },
              "start": {
                "character": 6,
                "line": 0,
              },
            },
            "severity": 1,
            "source": "oxlint-plugins",
          },
          {
            "code": "no-forbidden-identifiers/no-forbidden-identifiers",
            "data": {
              "identifier": "error",
              "ruleId": "no-forbidden-identifiers/no-forbidden-identifiers",
            },
            "message": "Identifier 'error' is reserved by this project policy.",
            "range": {
              "end": {
                "character": 24,
                "line": 0,
              },
              "start": {
                "character": 19,
                "line": 0,
              },
            },
            "severity": 1,
            "source": "oxlint-plugins",
          },
          {
            "code": "no-forbidden-identifiers/no-forbidden-identifiers",
            "data": {
              "identifier": "data",
              "ruleId": "no-forbidden-identifiers/no-forbidden-identifiers",
            },
            "message": "Identifier 'data' is reserved by this project policy.",
            "range": {
              "end": {
                "character": 18,
                "line": 0,
              },
              "start": {
                "character": 14,
                "line": 0,
              },
            },
            "severity": 1,
            "source": "oxlint-plugins",
          },
        ],
        "quickFix": {
          "diagnostics": [
            {
              "code": "no-forbidden-identifiers/no-forbidden-identifiers",
              "data": {
                "identifier": "event",
                "ruleId": "no-forbidden-identifiers/no-forbidden-identifiers",
              },
              "message": "Identifier 'event' is reserved by this project policy.",
              "range": {
                "end": {
                  "character": 11,
                  "line": 0,
                },
                "start": {
                  "character": 6,
                  "line": 0,
                },
              },
              "severity": 1,
              "source": "oxlint-plugins",
            },
          ],
          "edit": {
            "changes": {
              "file:///fixture.ts": [
                {
                  "newText": "reservedEvent",
                  "range": {
                    "end": {
                      "character": 11,
                      "line": 0,
                    },
                    "start": {
                      "character": 6,
                      "line": 0,
                    },
                  },
                },
              ],
            },
          },
          "kind": "quickfix",
          "title": "Replace event with reservedEvent",
        },
        "updated": "const reservedEvent = data.error;
      ",
      }
    `);
  });
});
