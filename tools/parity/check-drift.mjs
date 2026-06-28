// Parity corpus drift gate.
//
// Regenerates every committed corpus from the pinned upstream submodules and fails if the
// working tree changed — i.e. the committed ground truth no longer matches what the pinned
// upstream rule actually produces. Run this after bumping a submodule pin (regenerate +
// commit) and in the dedicated CI job that checks out submodules. It is intentionally NOT
// part of `pnpm run verify`, which stays hermetic (committed corpora + built port only).
//
//   pnpm run parity:drift

import { execFileSync } from 'node:child_process';

function run(cmd, args) {
  console.log(`$ ${cmd} ${args.join(' ')}`);
  execFileSync(cmd, args, { stdio: 'inherit' });
}

// CJS upstreams (plain require()).
run('node', ['tools/parity/capture/run.cjs', 'eslint-plugin-eslint-comments']);

// ESM / transform-required upstreams (loaded under vitest).
run('pnpm', ['exec', 'vitest', 'run', '--config', 'tools/parity/capture/vitest.config.ts']);

// Any change to committed corpora means drift from the pinned upstream.
try {
  run('git', ['diff', '--exit-code', '--', 'tools/parity/corpora']);
} catch {
  console.error(
    '\n✖ parity corpora drift: regenerated corpora differ from the committed copies.\n' +
      '  Either an upstream submodule pin moved without regenerating, or a corpus was hand-edited.\n' +
      '  Regenerate with `pnpm run parity:capture <plugin>` / `pnpm run parity:capture:esm` and commit.',
  );
  process.exit(1);
}

console.log('\n✔ parity corpora match the pinned upstream submodules.');
