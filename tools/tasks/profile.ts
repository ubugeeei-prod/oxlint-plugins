import { spawnSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

type ProfileTarget = 'bench' | 'js' | 'rust';

const target = (process.argv[2] ?? 'bench') as ProfileTarget;
const profilesDir = resolve('profiles');
mkdirSync(profilesDir, { recursive: true });

if (target === 'rust') {
  run('cargo', ['build', '--workspace', '--profile', 'profiling']);
} else if (target === 'bench') {
  run('cargo', ['bench', '--workspace', '--profile', 'profiling']);
} else if (target === 'js') {
  run('node', [
    '--cpu-prof',
    '--cpu-prof-dir',
    profilesDir,
    'node_modules/vitest/vitest.mjs',
    'run',
  ]);
} else {
  console.error('Usage: vp run profile <bench|rust|js>');
  process.exit(1);
}

function run(command: string, args: string[]): void {
  const result = spawnSync(command, args, {
    stdio: 'inherit',
  });

  process.exit(result.status ?? 1);
}
