// Captures the upstream eslint-plugin-regexp test suite straight from the
// vendored submodule and writes it to committed JSON fixtures, so our Vitest
// suite can replay the real upstream cases and track behavior as the submodule
// is bumped (oxc-style test syncing).
//
// The upstream tests drive cases through `SnapshotRuleTester` from
// `eslint-snapshot-rule-tester`. We register ESM module hooks
// (`module.registerHooks`) that stub the two specifiers the test files import:
//   - `eslint-snapshot-rule-tester` → a capturing stub that records
//     `{ name, valid, invalid }` on `globalThis[CAPTURE_KEY]`.
//   - anything containing `/lib/rules/` → `export default {};`
// Each upstream test file is copied to a temp dir before being dynamically
// imported, because bare specifiers resolved in-place under the submodule
// bypass the hook chain on Node 24.
//
// Cases that carry an `options` property cannot be replayed through the native
// `scanRegexp(sourceText, filename)` API (which runs all rules with default
// options); they are dropped and the count is logged (no silent truncation).
//
// Expected diagnostics are parsed from the companion `.eslintsnap` snapshot
// files. Each snapshot block corresponds 1:1 (in order) to an entry in the
// `invalid` array, so after dropping option-bearing invalid cases we zip the
// remaining invalid entries with their parsed blocks.
//
// Re-run with `pnpm run port:tests:regexp`.

import { registerHooks } from 'node:module';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Manifest = {
  submoduleRoot: string;
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    baselineVersion: string;
    license: string;
  }>;
};

type StatusEntry = {
  directory: string;
  rules: Array<{ name: string; status: string }>;
};

type RawCase = Record<string, unknown> | string;
type CapturedTests = { name: string; valid: RawCase[]; invalid: RawCase[] };

type ErrorEntry = {
  message: string;
  line: number;
  column: number;
  endColumn: number;
};

type InvalidFixture = {
  code: string;
  filename?: string;
  errorCount: number;
  errors: ErrorEntry[];
  output: string | null;
};

type ValidFixture = {
  code: string;
  filename?: string;
};

// ---------------------------------------------------------------------------
// Constants / config
// ---------------------------------------------------------------------------

const ROOT = process.cwd();
const HERE = dirname(fileURLToPath(import.meta.url));

// `SnapshotRuleTester.run` from the stub writes captured cases here, keyed on
// a shared global so the value crosses the hook-loaded ESM module boundary.
const CAPTURE_KEY = '__regexpSyncCapture__';

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;

const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-regexp');
if (!plugin) {
  throw new Error('eslint-plugin-regexp is not registered in tools/port-targets.json');
}

const SUBMODULE = join(ROOT, plugin.submodule);
const TESTS_DIR = join(SUBMODULE, 'tests', 'lib', 'rules');
const SNAPSHOTS_DIR = join(TESTS_DIR, '__snapshots__');
const FIXTURES_DIR = join(ROOT, 'npm', 'regexp', 'test', 'fixtures');

// ---------------------------------------------------------------------------
// Derive ported rules from status.json
// ---------------------------------------------------------------------------

function getPortedRules(): string[] {
  const statusPath = join(ROOT, 'status.json');
  if (!existsSync(statusPath)) {
    throw new Error(`status.json not found at ${statusPath}`);
  }
  const status = JSON.parse(readFileSync(statusPath, 'utf8')) as StatusEntry[];
  const entry = status.find((e) => e.directory === 'npm/regexp');
  if (!entry) {
    throw new Error('No entry with directory === "npm/regexp" found in status.json');
  }
  const ported = entry.rules.filter((r) => r.status === 'ported').map((r) => r.name);
  if (ported.length === 0) {
    throw new Error('No ported rules found for npm/regexp in status.json');
  }
  return ported.sort();
}

// ---------------------------------------------------------------------------
// ESM stub hooks (registered once at startup)
// ---------------------------------------------------------------------------

function registerStubHooks(): void {
  const captureStub = [
    `const CAP = '${CAPTURE_KEY}';`,
    'export class SnapshotRuleTester {',
    '  constructor() {}',
    '  run(name, rule, tests) {',
    '    globalThis[CAP] = {',
    '      name,',
    '      valid: (tests && tests.valid) || [],',
    '      invalid: (tests && tests.invalid) || [],',
    '    };',
    '  }',
    '}',
  ].join('\n');

  const ruleStub = 'export default {};';
  // Minimal eslint stub: ESLint and Linter classes are referenced in some test
  // files but the values are only used as parser options that the stub ignores.
  const eslintStub = [
    'export class ESLint {}',
    'export class Linter { getRules() { return new Map(); } }',
    'export const builtinRules = new Map();',
  ].join('\n');
  // @typescript-eslint/parser is imported and passed as a `parser` option on
  // some test cases. The stub only needs to be importable.
  const tsParserStub =
    'export default {}; export const parse = () => ({ type: "Program", body: [], range: [0,0] });';
  // Semver stub: provides enough of the semver API for version guards used in
  // test files. `gte(v, range)` for ESLint version gates (ESLint.version ===
  // undefined in our stub, so all version-gated cases default to false to keep
  // the test arrays stable). We return false for `gte` so extra ES2025 cases
  // are not included (they would appear in the invalid array but have no
  // corresponding snapshot block since the snapshots were generated with the
  // real ESLint version that may or may not include them).
  const semverStub = [
    'export default {',
    '  satisfies() { return false; },',
    '  gte() { return false; },',
    '  lt() { return false; },',
    '  lte() { return false; },',
    '  gt() { return false; },',
    '};',
  ].join('\n');
  const allRulesStub = 'export const rules = {};';

  registerHooks({
    resolve(specifier, _context, nextResolve) {
      if (specifier === 'eslint-snapshot-rule-tester') {
        return { url: 'stub:///capture', shortCircuit: true };
      }
      if (specifier === 'eslint' || specifier.startsWith('eslint/')) {
        return { url: 'stub:///eslint', shortCircuit: true };
      }
      if (specifier === '@typescript-eslint/parser') {
        return { url: 'stub:///ts-parser', shortCircuit: true };
      }
      if (specifier === 'semver') {
        return { url: 'stub:///semver', shortCircuit: true };
      }
      if (specifier.includes('/lib/rules/')) {
        return { url: 'stub:///rule', shortCircuit: true };
      }
      if (specifier.includes('/all-rules')) {
        return { url: 'stub:///all-rules', shortCircuit: true };
      }
      return nextResolve(specifier, _context);
    },
    load(url, _context, nextLoad) {
      if (url === 'stub:///capture') {
        return { format: 'module', source: captureStub, shortCircuit: true };
      }
      if (url === 'stub:///rule') {
        return { format: 'module', source: ruleStub, shortCircuit: true };
      }
      if (url === 'stub:///eslint') {
        return { format: 'module', source: eslintStub, shortCircuit: true };
      }
      if (url === 'stub:///ts-parser') {
        return { format: 'module', source: tsParserStub, shortCircuit: true };
      }
      if (url === 'stub:///semver') {
        return { format: 'module', source: semverStub, shortCircuit: true };
      }
      if (url === 'stub:///all-rules') {
        return { format: 'module', source: allRulesStub, shortCircuit: true };
      }
      return nextLoad(url, _context);
    },
  });
}

// ---------------------------------------------------------------------------
// Capture valid/invalid arrays from a test file via dynamic import
// ---------------------------------------------------------------------------

// Noop mocha globals so test files that call describe()/it()/before()/after()
// at module top level don't throw when imported outside a mocha context.
function injectMochaGlobals(): void {
  const g = globalThis as Record<string, unknown>;
  const noop = () => {};
  const suiteNoop = (_label: unknown, fn: () => void) => {
    try {
      fn();
    } catch {
      /* ignore */
    }
  };
  if (!g['describe']) g['describe'] = suiteNoop;
  if (!g['it']) g['it'] = noop;
  if (!g['before']) g['before'] = noop;
  if (!g['after']) g['after'] = noop;
  if (!g['beforeEach']) g['beforeEach'] = noop;
  if (!g['afterEach']) g['afterEach'] = noop;
}

async function captureTests(
  rule: string,
  testFile: string,
  tempDir: string,
): Promise<CapturedTests> {
  // Copy into a temp dir so that bare specifiers (eslint-snapshot-rule-tester)
  // are resolved relative to the temp file and hit the hook chain on Node 24.
  const tempFile = join(tempDir, `${rule}.ts`);
  writeFileSync(tempFile, readFileSync(testFile, 'utf8'));

  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = null;

  // Use a cache-busting query param so re-runs don't get a stale module.
  await import(`${tempFile}?t=${Date.now()}`);

  const captured = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedTests | null;
  if (!captured || !captured.name) {
    throw new Error(`No SnapshotRuleTester.run() call captured from ${testFile}`);
  }
  return captured;
}

// ---------------------------------------------------------------------------
// .eslintsnap parser
// ---------------------------------------------------------------------------

// Split the snapshot content into per-invalid blocks.
function parseSnapshotBlocks(snapText: string): string[] {
  // Drop the header line then split on `---` separator lines.
  const withoutHeader = snapText.replace(/^# eslint-snapshot-rule-tester format: v1\n/, '');
  return withoutHeader
    .split(/\n---\n/)
    .map((block) => block.trim())
    .filter(Boolean);
}

// Given a single snapshot block, extract errors and output.
function parseBlock(block: string): { errors: ErrorEntry[]; output: string | null } {
  const lines = block.split('\n');

  // Collect Code: source + marker lines for location computation.
  // Also collect top-level message lines `[n] text`.
  // Top-level Output: lines (not indented).

  let inCode = false;
  let inOutput = false;
  let inSuggestions = false;

  // Source lines in the Code: section: { lineNo, content, absIndex } where
  // absIndex is the index in the full line where source content begins (after
  // `  N | `).
  const sourceLines: Array<{ lineNo: number; content: string; sourceStart: number }> = [];
  // Marker lines (in order) in the Code: section.
  const markerLines: Array<{ raw: string; sourceStart: number }> = [];
  // Output source lines for top-level (non-suggestion) Output.
  const outputLines: string[] = [];
  const messageLines: string[] = [];

  for (const line of lines) {
    // Section headers
    if (line === 'Code:') {
      inCode = true;
      inOutput = false;
      inSuggestions = false;
      continue;
    }
    if (line === 'Output:') {
      // Top-level Output: (zero indentation)
      inCode = false;
      inOutput = true;
      inSuggestions = false;
      continue;
    }
    if (line === 'Output: unchanged') {
      inCode = false;
      inOutput = false;
      inSuggestions = false;
      continue;
    }
    if (line === '    Suggestions:' || line.startsWith('    Suggestions:')) {
      inSuggestions = true;
      inCode = false;
      inOutput = false;
      continue;
    }
    // Skip lines that are part of Suggestions block (they're indented >= 6 spaces)
    if (inSuggestions) {
      continue;
    }

    if (inCode) {
      // Source line: `  N | source` or `  N |` (empty source line)
      const srcMatch = /^  (\d+) \| ?(.*)$/.exec(line);
      if (srcMatch) {
        const lineNo = parseInt(srcMatch[1], 10);
        // sourceStart is the position (in the full line) where source starts.
        // Format: `  {lineNo} | {source}` — source starts right after `| `.
        // For empty lines `  N |`, use pipeIdx+2 as a consistent offset.
        const pipeIdx = line.indexOf('|');
        const sourceStart = pipeIdx + 2;
        sourceLines.push({ lineNo, content: srcMatch[2], sourceStart });
        continue;
      }
      // Marker line: `    | markers`
      const markerMatch = /^    \| (.*)$/.exec(line);
      if (markerMatch) {
        // sourceStart: same offset as source lines. The `| ` in marker lines
        // starts at position 4 (the `    |`), so content starts at 6.
        const pipeIdx = line.indexOf('| ');
        const sourceStart = pipeIdx + 2;
        markerLines.push({ raw: line, sourceStart });
        continue;
      }
      // Blank line inside Code: — end of Code section (shouldn't happen but be safe)
      if (line === '') {
        inCode = false;
      }
      continue;
    }

    if (inOutput) {
      // Output source line: `  N | content` or `  N |` (empty content line)
      const srcMatch = /^  (\d+) \| ?(.*)$/.exec(line);
      if (srcMatch) {
        outputLines.push(srcMatch[2]);
        continue;
      }
      // Blank line ends output
      if (line === '') {
        inOutput = false;
      }
      continue;
    }

    // Top-level message lines: `[n] text`
    if (/^\[\d+\]/.test(line)) {
      messageLines.push(line);
    }
  }

  // Parse message lines into { index, message }
  const messages: Array<{ index: number; message: string }> = [];
  for (const ml of messageLines) {
    const m = /^\[(\d+)\] (.*)$/.exec(ml);
    if (m) {
      messages.push({ index: parseInt(m[1], 10), message: m[2] });
    }
  }

  // Parse markers to extract error locations.
  // Strategy:
  //   For each marker line, scan for `^[~]*` runs. Each run may be tagged
  //   with `[n]` either:
  //     a) Inline: `^~~~~ [1]` — tag immediately follows the `~*` with a space.
  //     b) On the next marker line: a line that contains only `[n]` labels
  //        aligned under the carets (no `^` or `~`).
  //
  // We process markers in two passes:
  //   1. Collect all caret runs from all marker lines (with their source line ref).
  //   2. For each run, determine its `[n]` tag (inline or from a label-only line).

  // Each parsed marker: { n, line (source), col1Based, endCol1Based }
  const parsedMarkers: Map<number, ErrorEntry> = new Map();

  // Collect caret runs in order. We need to track which source line each
  // marker run applies to.
  type CaretRun = {
    n: number | null; // null means read from label line
    sourceLine: number;
    col1Based: number;
    endCol1Based: number;
    markerLineIdx: number;
    colInMarkerLine: number; // abs col of the `^` for label matching
  };

  const allCaretRuns: CaretRun[] = [];

  // For each marker line, find the most recent source line above it.
  // We need to track them in order.
  // Rebuild a combined list of code-section lines in order.
  // Actually we already have sourceLines and markerLines collected in order.
  // We need to correlate: each markerLine should be associated with the most
  // recent sourceLines entry that appeared before it.
  //
  // Re-parse the Code: section lines in order to maintain the association.

  let currentSourceLine: { lineNo: number; sourceStart: number } | null = null;
  let codeStarted = false;
  let markerLineIdx = 0;

  // Build ordered list of code section events
  type CodeEvent =
    | { type: 'source'; lineNo: number; sourceStart: number }
    | { type: 'marker'; raw: string; sourceStart: number; idx: number };

  const codeEvents: CodeEvent[] = [];
  let mIdx = 0;
  for (const line of lines) {
    if (line === 'Code:') {
      codeStarted = true;
      continue;
    }
    if (
      codeStarted &&
      (line === 'Output:' ||
        line === 'Output: unchanged' ||
        line === '' ||
        line === '    Suggestions:' ||
        line.startsWith('    Suggestions:'))
    ) {
      codeStarted = false;
      continue;
    }
    if (!codeStarted) continue;

    const srcMatch = /^  (\d+) \| ?(.*)$/.exec(line);
    if (srcMatch) {
      const lineNo = parseInt(srcMatch[1], 10);
      const pipeIdx = line.indexOf('|');
      codeEvents.push({ type: 'source', lineNo, sourceStart: pipeIdx + 2 });
      continue;
    }
    const markerMatch = /^    \| (.*)$/.exec(line);
    if (markerMatch) {
      const pipeIdx = line.indexOf('| ');
      codeEvents.push({ type: 'marker', raw: line, sourceStart: pipeIdx + 2, idx: mIdx++ });
    }
  }

  // Now process code events to build caret runs
  let lastSourceLineNo = 1;
  let lastSourceStart = 6; // default

  for (let i = 0; i < codeEvents.length; i++) {
    const evt = codeEvents[i];
    if (evt.type === 'source') {
      lastSourceLineNo = evt.lineNo;
      lastSourceStart = evt.sourceStart;
      continue;
    }

    // marker line
    const raw = evt.raw;
    const mStart = evt.sourceStart; // absolute col where markers content begins

    // Find all caret runs on this marker line
    const content = raw.slice(mStart); // the marker content after `    | `

    // Scan for `^[~]*` patterns with optional inline `[n]` tag
    const caretPattern = /\^(~*)(\s+\[(\d+)\])?/g;
    let match: RegExpExecArray | null;
    while ((match = caretPattern.exec(content)) !== null) {
      const caretAbsCol = mStart + match.index; // 0-based absolute column in the full line
      const tildes = match[1];
      const inlineN = match[3] ? parseInt(match[3], 10) : null;

      // col1Based = caretAbsCol - mStart + 1 (1-based relative to source start)
      // Wait: the instruction says: `sourceColumn1Based = caretAbsoluteIndex - (pipeIndex + 2) + 1`
      // caretAbsoluteIndex is the position in the full raw line where `^` appears.
      // The caret's position in the full `raw` string is mStart + match.index.
      // pipeIndex + 2 = mStart (that's what we computed).
      // So col1Based = (mStart + match.index) - mStart + 1 = match.index + 1.
      const col1Based = match.index + 1;
      const endCol1Based = match.index + 1 + tildes.length + 1; // exclusive, after all ^~s

      allCaretRuns.push({
        n: inlineN,
        sourceLine: lastSourceLineNo,
        col1Based,
        endCol1Based,
        markerLineIdx: i,
        colInMarkerLine: mStart + match.index,
      });
    }

    // Check if this marker line itself is a label-only line (no `^` or `~`)
    // E.g. `    |             [1]    [2]`
    // These are handled below when we match label lines to preceding caret runs.
  }

  // Now handle label-only lines: find marker lines that have `[n]` but no `^`
  // These are label lines that annotate the immediately preceding caret-run line.
  for (let i = 0; i < codeEvents.length; i++) {
    const evt = codeEvents[i];
    if (evt.type !== 'marker') continue;

    const raw = evt.raw;
    const mStart = evt.sourceStart;
    const content = raw.slice(mStart);

    // If this line has `^` it's a caret line, not a pure label line
    if (content.includes('^')) continue;

    // Check if it has `[n]` labels
    const labelPattern = /\[(\d+)\]/g;
    let labelMatch: RegExpExecArray | null;
    const labels: Array<{ n: number; col: number }> = [];
    while ((labelMatch = labelPattern.exec(content)) !== null) {
      labels.push({ n: parseInt(labelMatch[1], 10), col: mStart + labelMatch.index });
    }
    if (labels.length === 0) continue;

    // This is a label-only line. Find caret runs from the preceding marker line(s)
    // that have n === null (i.e., no inline tag), and match them by column alignment.
    // The label `[n]` is aligned under the `^` of the corresponding caret run.
    // We look at the preceding caret-run marker line.
    for (const label of labels) {
      // Find the untagged caret run whose `^` position is at or just before this label's `[`
      // The `[` of the label is aligned to be at or after the `^`.
      // Match: find untagged run on the immediately preceding caret-line where
      // colInMarkerLine <= label.col <= colInMarkerLine + span
      for (const run of allCaretRuns) {
        if (run.n !== null) continue;
        // Check that this run is from a marker line that precedes this label line
        // (markerLineIdx < i)
        if (run.markerLineIdx >= i) continue;

        // The label column should be within or after the caret span
        // label.col is the absolute column of `[` in the raw line
        const runEndAbsCol = run.colInMarkerLine + (run.endCol1Based - run.col1Based); // length of ^~~~~
        // Allow the label `[` to be up to 2 positions to the LEFT of the `^`
        // because the snapshot renderer sometimes places short labels like `[1]`
        // one column before the caret (e.g. `    ^~ ^~~~~` / `   [1] [2]`).
        const span = run.endCol1Based - run.col1Based;
        if (label.col >= run.colInMarkerLine - 2 && label.col <= run.colInMarkerLine + span + 4) {
          run.n = label.n;
          break;
        }
      }
    }
  }

  // Build error map from caret runs
  for (const run of allCaretRuns) {
    if (run.n === null) {
      // Couldn't match — might be a complex multi-row case; skip gracefully
      continue;
    }
    if (!parsedMarkers.has(run.n)) {
      // Find message
      const msgEntry = messages.find((m) => m.index === run.n);
      const message = msgEntry ? msgEntry.message : '';
      parsedMarkers.set(run.n, {
        message,
        line: run.sourceLine,
        column: run.col1Based,
        endColumn: run.endCol1Based,
      });
    }
  }

  // Build errors array in order of message index
  const errors: ErrorEntry[] = [];
  for (const msg of messages) {
    const marker = parsedMarkers.get(msg.index);
    if (marker) {
      errors.push(marker);
    } else {
      // No location found — still include the error with message only
      errors.push({
        message: msg.message,
        line: 0,
        column: 0,
        endColumn: 0,
      });
    }
  }

  const output = outputLines.length > 0 ? outputLines.join('\n') : null;

  return { errors, output };
}

// ---------------------------------------------------------------------------
// Normalize a raw test case (drop options-bearing cases)
// ---------------------------------------------------------------------------

function normalizeValidCase(raw: RawCase): ValidFixture | null {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (value == null || typeof value !== 'object') return null;

  // Drop cases with options (can't replay with default-options-only API)
  if (
    'options' in value &&
    Array.isArray(value.options) &&
    (value.options as unknown[]).length > 0
  ) {
    return null;
  }

  try {
    const clone = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    if (!('code' in clone) || typeof clone.code !== 'string') return null;
    const result: ValidFixture = { code: clone.code };
    if (typeof clone.filename === 'string') result.filename = clone.filename;
    return result;
  } catch {
    return null;
  }
}

function normalizeInvalidCase(raw: RawCase): (Record<string, unknown> & { code: string }) | null {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (value == null || typeof value !== 'object') return null;

  // Drop cases with options
  if (
    'options' in value &&
    Array.isArray(value.options) &&
    (value.options as unknown[]).length > 0
  ) {
    return null;
  }

  try {
    const clone = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    if (!('code' in clone) || typeof clone.code !== 'string') return null;
    return clone as Record<string, unknown> & { code: string };
  } catch {
    return null;
  }
}

function isPresent<T>(value: T | null): value is T {
  return value != null;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  if (!existsSync(TESTS_DIR)) {
    throw new Error(
      `Upstream tests not found at ${TESTS_DIR}. Run: git submodule update --init --depth 1 ${plugin!.submodule}`,
    );
  }

  mkdirSync(FIXTURES_DIR, { recursive: true });

  registerStubHooks();
  injectMochaGlobals();

  const tempDir = mkdtempSync(join(tmpdir(), 'regexp-sync-'));

  try {
    const portedRules = getPortedRules();
    const summary: string[] = [];
    let totalFixtures = 0;
    let totalDropped = 0;

    for (const rule of portedRules) {
      const testFile = join(TESTS_DIR, `${rule}.ts`);
      if (!existsSync(testFile)) {
        throw new Error(`Upstream test file missing for rule "${rule}": ${testFile}`);
      }

      const snapFile = join(SNAPSHOTS_DIR, `${rule}.ts.eslintsnap`);
      if (!existsSync(snapFile)) {
        throw new Error(`Upstream snapshot file missing for rule "${rule}": ${snapFile}`);
      }

      // Capture valid/invalid arrays
      const captured = await captureTests(rule, testFile, tempDir);

      // Parse ALL snapshot blocks (1:1 with captured.invalid)
      const snapText = readFileSync(snapFile, 'utf8');
      const allBlocks = parseSnapshotBlocks(snapText);

      if (allBlocks.length !== captured.invalid.length) {
        // Log a warning but don't crash — just note the mismatch
        console.warn(
          `  WARN ${rule}: snapshot blocks (${allBlocks.length}) !== invalid cases (${captured.invalid.length}); proceeding with zip`,
        );
      }

      // Zip invalid cases with snapshot blocks, then filter option-bearing cases
      type Paired = { raw: RawCase; block: string };
      const paired: Paired[] = captured.invalid.map((raw, i) => ({
        raw,
        block: allBlocks[i] ?? '',
      }));

      const validRaw = captured.valid;
      const validNormalized: ValidFixture[] = [];
      let validDropped = 0;
      for (const raw of validRaw) {
        const normalized = normalizeValidCase(raw);
        if (normalized) {
          validNormalized.push(normalized);
        } else {
          validDropped++;
        }
      }

      const invalidNormalized: InvalidFixture[] = [];
      let invalidDropped = 0;
      for (const { raw, block } of paired) {
        const normalized = normalizeInvalidCase(raw);
        if (!normalized) {
          invalidDropped++;
          continue;
        }

        let errors: ErrorEntry[] = [];
        let output: string | null = null;
        if (block) {
          const parsed = parseBlock(block);
          errors = parsed.errors;
          output = parsed.output;
        }

        const entry: InvalidFixture = {
          code: normalized.code,
          errorCount: errors.length,
          errors,
          output,
        };
        if (typeof normalized.filename === 'string') {
          entry.filename = normalized.filename;
        }
        invalidNormalized.push(entry);
      }

      const dropped = validDropped + invalidDropped;
      totalDropped += dropped;
      totalFixtures++;

      const fixture = {
        __generated: {
          source: plugin!.npm,
          version: plugin!.baselineVersion,
          sourceFile: `tests/lib/rules/${rule}.ts`,
          snapshotFile: `tests/lib/rules/__snapshots__/${rule}.ts.eslintsnap`,
          license: plugin!.license,
          tool: 'tools/tasks/sync-regexp-tests.ts',
        },
        valid: validNormalized,
        invalid: invalidNormalized,
      };

      writeFileSync(join(FIXTURES_DIR, `${rule}.json`), `${JSON.stringify(fixture, null, 2)}\n`);

      summary.push(
        `${rule}: ${validNormalized.length} valid, ${invalidNormalized.length} invalid${
          dropped > 0 ? ` (${dropped} option case(s) dropped)` : ''
        }`,
      );
    }

    console.log('Synced regexp fixtures from upstream:');
    for (const line of summary) {
      console.log(`  - ${line}`);
    }
    console.log(
      `\nTotal: ${totalFixtures} fixture file(s) generated, ${totalDropped} option case(s) dropped`,
    );
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

void HERE;
