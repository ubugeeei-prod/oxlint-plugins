import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const ruleName = 'only-export-components';
const rule = plugin.rules[ruleName];

function runRule({ code, filename = 'Component.tsx', options = [] }) {
  const reports = [];
  const visitor = rule.createOnce({
    filename,
    options,
    sourceCode: {
      getText() {
        return code;
      },
    },
    report(descriptor) {
      reports.push({
        message: rule.meta.messages[descriptor.messageId],
        messageId: descriptor.messageId,
        loc: descriptor.loc,
      });
    },
  });

  visitor.Program({ type: 'Program' });
  return reports;
}

function toLspDiagnostics(reports) {
  return reports.map((report) => ({
    range: {
      start: {
        line: report.loc.start.line - 1,
        character: report.loc.start.column,
      },
      end: {
        line: report.loc.end.line - 1,
        character: report.loc.end.column,
      },
    },
    severity: 1,
    source: 'oxlint-plugins',
    code: `react-refresh/${ruleName}`,
    message: report.message,
  }));
}

describe('react-refresh LSP diagnostics fixture', () => {
  it('maps a non-component export diagnostic', () => {
    const reports = runRule({
      code: 'export const Foo = () => null;\nexport const foo = 1;\n',
    });

    expect(toLspDiagnostics(reports)).toMatchInlineSnapshot(`
      [
        {
          "code": "react-refresh/only-export-components",
          "message": "Fast refresh only works when a file only exports components. Use a new file to share constants or functions between components.",
          "range": {
            "end": {
              "character": 16,
              "line": 1,
            },
            "start": {
              "character": 13,
              "line": 1,
            },
          },
          "severity": 1,
          "source": "oxlint-plugins",
        },
      ]
    `);
  });

  it('maps a local component without exports diagnostic', () => {
    const reports = runRule({
      code: 'const Foo = () => null;\n',
    });

    expect(toLspDiagnostics(reports)).toMatchInlineSnapshot(`
      [
        {
          "code": "react-refresh/only-export-components",
          "message": "Fast refresh only works when a file has exports. Move your component(s) to a separate file.",
          "range": {
            "end": {
              "character": 9,
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
      ]
    `);
  });
});
