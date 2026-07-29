// Captures the stable @stylistic/eslint-plugin JSX parser matrix as a
// committed fixture and enriches each invalid case with the exact report and
// first-pass fix produced by the pinned rule.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = {
  name: string;
  valid: RawCase[];
  invalid: RawCase[];
};

const ROOT = process.cwd();
const UPSTREAM_REF = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const RULE = 'jsx-closing-tag-location';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const RULE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.ts`;
const PARSER_MATRIX_FILE = 'shared/test-utils/parsers-jsx.ts';
const FIXTURES_DIR = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures');
const FIXTURE_FILE = join(FIXTURES_DIR, `${RULE}-v5.10.0.json`);
const CAPTURE_KEY = '__stylisticJsxClosingTagLocationCapture__';
const MESSAGES = {
  onOwnLine: 'Closing tag of a multiline JSX expression must be on its own line.',
  matchIndent: 'Expected closing tag to match indentation of opening.',
  alignWithOpening: 'Expected closing tag to be aligned with the line containing the opening tag',
} as const;

if (!existsSync(UPSTREAM_DIR)) {
  throw new Error(
    `Upstream checkout not found at ${UPSTREAM_DIR}. Run \`git submodule update --init upstream/eslint-stylistic\` first.`,
  );
}

const actualCommit = execFileSync('git', ['-C', UPSTREAM_DIR, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== UPSTREAM_COMMIT) {
  throw new Error(
    `Expected upstream/eslint-stylistic at ${UPSTREAM_COMMIT}, received ${actualCommit}.`,
  );
}

const source = readPinnedFile(SOURCE_FILE);
const ruleSource = readPinnedFile(RULE_FILE);
const parserMatrixSource = readPinnedFile(PARSER_MATRIX_FILE);
registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-closing-tag-location-sync-'));
const tempFile = join(tempDir, `${RULE}.test.ts`);

try {
  writeFileSync(tempFile, source);
  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  await import(`${pathToFileURL(tempFile).href}?capture=${Date.now()}`);
  const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
  if (runs.length !== 1 || runs[0].name !== RULE) {
    throw new Error(`Expected one captured ${RULE} suite, received ${runs.length}.`);
  }

  const valid = runs[0].valid.map(normalizeCase);
  const invalid = runs[0].invalid.map((testCase) => enrichInvalid(normalizeCase(testCase)));
  const diagnostics = invalid.reduce(
    (count, testCase) => count + (testCase.diagnostics as unknown[]).length,
    0,
  );
  const fixture = {
    __generated: {
      source: '@stylistic/eslint-plugin',
      version: UPSTREAM_REF,
      commit: UPSTREAM_COMMIT,
      sourceFile: SOURCE_FILE,
      ruleFile: RULE_FILE,
      parserMatrixFile: PARSER_MATRIX_FILE,
      sourceSha256: sha256(source),
      ruleSourceSha256: sha256(ruleSource),
      parserMatrixSourceSha256: sha256(parserMatrixSource),
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-jsx-closing-tag-location-tests.ts',
      inventory: {
        valid: valid.length,
        invalid: invalid.length,
        diagnostics,
        fixableInvalid: invalid.filter((testCase) => testCase.output !== null).length,
        unfixableInvalid: invalid.filter((testCase) => testCase.output === null).length,
        total: valid.length + invalid.length,
      },
    },
    valid,
    invalid,
  };

  mkdirSync(FIXTURES_DIR, { recursive: true });
  writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('vp', ['fmt', FIXTURE_FILE], { stdio: 'ignore' });
  console.log(
    `Synced ${RULE} from @stylistic/eslint-plugin ${UPSTREAM_REF}: ${valid.length} valid, ${invalid.length} invalid, ${diagnostics} diagnostics.`,
  );
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}

function readPinnedFile(file: string): string {
  return execFileSync('git', ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${file}`], {
    encoding: 'utf8',
  });
}

function normalizeCase(raw: RawCase): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  const clone = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
  if (typeof clone.code !== 'string') {
    throw new TypeError(`Captured ${RULE} case is missing string code.`);
  }
  return clone;
}

function enrichInvalid(testCase: Record<string, unknown>): Record<string, unknown> {
  const code = testCase.code as string;
  const errors = testCase.errors as Array<{ messageId: keyof typeof MESSAGES }>;
  if (errors.length !== 1) {
    throw new Error(`Expected one ${RULE} diagnostic, received ${errors.length}.`);
  }

  const closingStart = code.indexOf('</');
  const closingEnd = code.indexOf('>', closingStart) + 1;
  const openingStart = code.indexOf('<');
  if (openingStart < 0 || closingStart < 0 || closingEnd <= closingStart) {
    throw new Error(`Unable to locate JSX tags in captured ${RULE} case.`);
  }

  const messageId = errors[0].messageId;
  const openingLineStart = lineStart(code, openingStart);
  const closingLineStart = lineStart(code, closingStart);
  const openingColumn = code.slice(openingLineStart, openingStart).length;
  const openingIndent = /^\s*/u.exec(code.slice(openingLineStart))?.[0].length ?? 0;
  const indent =
    (testCase.options as string[] | undefined)?.[0] === 'line-aligned'
      ? openingIndent
      : openingColumn;
  const firstInLine = /^\s*$/u.test(code.slice(closingLineStart, closingStart));
  const fix = firstInLine
    ? {
        range: [closingLineStart, closingStart],
        replacementText: ' '.repeat(indent),
      }
    : {
        range: [closingStart, closingStart],
        replacementText: `\n${' '.repeat(indent)}`,
      };

  return {
    ...testCase,
    diagnostics: [
      {
        messageId,
        message: MESSAGES[messageId],
        data: {},
        ...locationRange(code, closingStart, closingEnd),
        range: [closingStart, closingEnd],
        fix,
      },
    ],
  };
}

function lineStart(source: string, offset: number): number {
  for (let index = offset; index > 0; index -= 1) {
    const character = source[index - 1];
    if (
      character === '\n' ||
      character === '\r' ||
      character === '\u2028' ||
      character === '\u2029'
    ) {
      return index;
    }
  }
  return 0;
}

function locationRange(source: string, start: number, end: number) {
  const startLocation = locationAt(source, start);
  const endLocation = locationAt(source, end);
  return {
    line: startLocation.line,
    column: startLocation.column,
    endLine: endLocation.line,
    endColumn: endLocation.column,
  };
}

function locationAt(source: string, offset: number) {
  let line = 1;
  let lineStart = 0;
  for (let index = 0; index < offset; index += 1) {
    const character = source[index];
    if (character === '\r') {
      if (source[index + 1] === '\n') {
        index += 1;
      }
      line += 1;
      lineStart = index + 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      line += 1;
      lineStart = index + 1;
    }
  }
  return { line, column: offset - lineStart + 1 };
}

function sha256(value: string): string {
  return createHash('sha256').update(value).digest('hex');
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = '${CAPTURE_KEY}';`,
    'export function run(options) {',
    '  globalThis[key].push({',
    '    name: options.name,',
    '    valid: options.valid || [],',
    '    invalid: options.invalid || [],',
    '  });',
    '}',
  ].join('\n');
  const parsersStub = `
    const BABEL_ESLINT = function babelParser() {};
    const TYPESCRIPT_ESLINT = function typescriptParser() {};

    function babelParserOptions(test, features) {
      return Object.assign({}, test.parserOptions, {
        requireConfigFile: false,
        babelOptions: {
          presets: ['@babel/preset-react'],
          plugins: [
            '@babel/plugin-syntax-do-expressions',
            '@babel/plugin-syntax-function-bind',
            ['@babel/plugin-syntax-decorators', { legacy: true }],
          ],
          parserOpts: {
            allowSuperOutsideMethod: false,
            allowReturnOutsideFunction: false,
          },
        },
        ecmaFeatures: Object.assign({}, test.parserOptions && test.parserOptions.ecmaFeatures, {
          jsx: true,
          modules: true,
          legacyDecorators: features.has('decorators'),
        }),
      });
    }

    function tsParserOptions(test, features) {
      return {
        ...test.parserOptions,
        ecmaFeatures: {
          jsx: true,
          modules: true,
          legacyDecorators: features.has('decorators'),
        },
      };
    }

    function applyAllParsers(tests) {
      return tests.flatMap((raw) => {
        const test = typeof raw === 'string' ? { code: raw } : { ...raw };
        const features = new Set(test.features || []);
        delete test.features;

        function addComment(testObject, parser) {
          const extras = [
            \`features: [\${Array.from(features).join(',')}]\`,
            \`parser: \${parser}\`,
            testObject.parserOptions
              ? \`parserOptions: \${JSON.stringify(testObject.parserOptions)}\`
              : '',
            testObject.options ? \`options: \${JSON.stringify(testObject.options)}\` : '',
            testObject.settings ? \`settings: \${JSON.stringify(testObject.settings)}\` : '',
          ];
          const extraComment = \`\\n// \${extras.join(',')}\`;
          const nextErrors =
            testObject.errors && typeof testObject.errors !== 'number'
              ? {
                  errors: testObject.errors.map((error) => ({
                    ...error,
                    ...(error.suggestions
                      ? {
                          suggestions: error.suggestions.map((suggestion) => ({
                            ...suggestion,
                            output: suggestion.output + extraComment,
                          })),
                        }
                      : {}),
                  })),
                }
              : {};
          return Object.assign(
            {},
            testObject,
            { code: testObject.code + extraComment },
            testObject.output && { output: testObject.output + extraComment },
            nextErrors,
          );
        }

        const skipBase =
          features.has('class fields') ||
          features.has('no-default') ||
          features.has('bind operator') ||
          features.has('do expressions') ||
          features.has('decorators') ||
          features.has('flow') ||
          features.has('ts') ||
          features.has('types') ||
          features.has('fragment');
        const skipNewBabel =
          features.has('no-babel') ||
          features.has('no-babel-new') ||
          features.has('flow') ||
          features.has('types') ||
          features.has('ts');
        const skipTS =
          features.has('no-ts') ||
          features.has('flow') ||
          features.has('jsx namespace') ||
          features.has('bind operator') ||
          features.has('do expressions');

        return [
          ...(skipBase ? [] : [addComment(test, 'default')]),
          ...(skipNewBabel
            ? []
            : [
                addComment(
                  {
                    ...test,
                    languageOptions: {
                      parser: BABEL_ESLINT,
                      parserOptions: babelParserOptions(test, features),
                    },
                  },
                  '@babel/eslint-parser',
                ),
              ]),
          ...(skipTS || features.has('no-ts-new')
            ? []
            : [
                addComment(
                  {
                    ...test,
                    languageOptions: {
                      parser: TYPESCRIPT_ESLINT,
                      parserOptions: tsParserOptions(test, features),
                    },
                  },
                  '@typescript-eslint/parser',
                ),
              ]),
        ];
      });
    }

    export function valids(...tests) {
      return applyAllParsers(tests.flat().filter(Boolean));
    }

    export function invalids(...tests) {
      return applyAllParsers(tests.flat().filter(Boolean));
    }
  `;

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///test', shortCircuit: true };
      }
      if (specifier === '#test/parsers-jsx') {
        return { url: 'stub:///parsers-jsx', shortCircuit: true };
      }
      if (
        specifier === './jsx-closing-tag-location' ||
        specifier === './types' ||
        specifier === './types.d.ts'
      ) {
        return { url: 'stub:///rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///test') {
        return { format: 'module', source: testStub, shortCircuit: true };
      }
      if (url === 'stub:///parsers-jsx') {
        return { format: 'module', source: parsersStub, shortCircuit: true };
      }
      if (url === 'stub:///rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}
