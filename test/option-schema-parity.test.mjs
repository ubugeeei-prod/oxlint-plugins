// Regression guard: a plugin that advertises a rule options schema must
// actually read the user's `context.options` in its adapter.
//
// Background: several ports declared a permissive `{ additionalProperties: true }`
// schema (so ESLint/Oxlint accept user options) but never forwarded those
// options to the Rust core, so the configuration silently did nothing. This is
// the worst kind of gap — the config validates, then is ignored. testing-library
// fixed this for `consistent-data-testid` (#343); this test stops it recurring.
//
// The check is intentionally static (reads tracked source only — no native
// build required): if `index.js` declares an options-accepting schema shape
// (`additionalProperties: true` or `properties: { ... }`), it must also
// reference `context.options`.
//
// KNOWN_OPTION_GAPS lists ports whose Rust cores do not yet implement options
// at all; they are being given real option support in follow-up PRs and are
// exempted until then. Remove an entry once its adapter forwards options.

import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const npmDir = resolve(dirname(fileURLToPath(import.meta.url)), '..', 'npm');

// Ports whose cores ignore options entirely; tracked for real implementation.
// perfectionist: configurable sort engine not implemented.
// angular-eslint: per-rule options (selectors/prefixes/suffixes) not implemented.
const KNOWN_OPTION_GAPS = new Set(['perfectionist', 'angular-eslint']);

// Note: `playwright` passes this static check because it reads `context.options`
// as a presence gate, but its core drops the option *values*; that deeper gap is
// tracked separately and cannot be detected statically.

function read(path) {
  try {
    return readFileSync(path, 'utf8');
  } catch {
    return '';
  }
}

function declaresOptionSchema(indexJs) {
  return /additionalProperties:\s*true/.test(indexJs) || /\bproperties:\s*\{/.test(indexJs);
}

const plugins = readdirSync(npmDir, { withFileTypes: true })
  .filter((entry) => entry.isDirectory() && existsSync(join(npmDir, entry.name, 'index.js')))
  .map((entry) => entry.name);

describe('option schema parity', () => {
  it('finds plugin packages to check', () => {
    expect(plugins.length).toBeGreaterThan(10);
  });

  for (const plugin of plugins) {
    it(`${plugin}: declaring an options schema implies reading context.options`, () => {
      const indexJs = read(join(npmDir, plugin, 'index.js'));
      if (!declaresOptionSchema(indexJs)) {
        return; // No options advertised — nothing to forward.
      }

      const readsContextOptions = /context\.options/.test(indexJs);
      if (KNOWN_OPTION_GAPS.has(plugin)) {
        // Sanity: keep the allowlist honest. If a gap plugin starts reading
        // context.options it should be removed from KNOWN_OPTION_GAPS.
        expect(
          readsContextOptions,
          `${plugin} now reads context.options — remove it from KNOWN_OPTION_GAPS`,
        ).toBe(false);
        return;
      }

      expect(
        readsContextOptions,
        `${plugin} advertises a rule options schema but never reads context.options, ` +
          `so user options are silently ignored. Forward context.options to the scanner ` +
          `(see npm/testing-library for the pattern), or add ${plugin} to KNOWN_OPTION_GAPS ` +
          `if its core cannot honor options yet.`,
      ).toBe(true);
    });
  }
});
