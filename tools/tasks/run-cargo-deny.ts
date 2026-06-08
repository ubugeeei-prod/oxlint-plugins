import { spawnSync } from 'node:child_process';

const hasCargoDeny = spawnSync('cargo', ['deny', '--version'], { stdio: 'ignore' }).status === 0;

if (!hasCargoDeny) {
  const install = spawnSync('cargo', ['install', 'cargo-deny', '--locked'], { stdio: 'inherit' });
  if (install.status !== 0) {
    process.exit(install.status ?? 1);
  }
}

const result = spawnSync('cargo', ['deny', 'check', 'advisories', 'bans', 'licenses', 'sources'], {
  stdio: 'inherit',
});
process.exit(result.status ?? 1);
