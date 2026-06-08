import { existsSync, readFileSync } from 'node:fs';

type PackageJson = {
  name: string;
  version: string;
};
type PackageStatus = {
  packageName?: string;
  directory?: string;
  version?: string;
  rules?: unknown[];
};

const status = JSON.parse(readFileSync('status.json', 'utf8')) as PackageStatus[];
const failures: string[] = [];

for (const plugin of status) {
  if (!plugin.packageName || !plugin.directory || !plugin.version) {
    failures.push(`Invalid plugin entry: ${JSON.stringify(plugin)}`);
    continue;
  }

  const packagePath = `${plugin.directory}/package.json`;
  if (!existsSync(packagePath)) {
    failures.push(`${plugin.packageName}: missing ${packagePath}`);
    continue;
  }

  const pkg = JSON.parse(readFileSync(packagePath, 'utf8')) as PackageJson;
  if (pkg.name !== plugin.packageName) {
    failures.push(
      `${packagePath}: package name ${pkg.name} does not match status ${plugin.packageName}`,
    );
  }
  if (pkg.version !== plugin.version) {
    failures.push(
      `${packagePath}: package version ${pkg.version} does not match status ${plugin.version}`,
    );
  }
  if (!Array.isArray(plugin.rules) || plugin.rules.length === 0) {
    failures.push(`${plugin.packageName}: expected at least one rule status entry`);
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(failure);
  }
  process.exit(1);
}

console.log('Package status is consistent.');
