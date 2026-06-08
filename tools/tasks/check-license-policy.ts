import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

type LicenseMetadata = string | { type?: string };
type PackageJson = {
  name?: string;
  license?: LicenseMetadata;
};
type LicenseException = {
  package: string;
  license: string;
};

const exceptions = JSON.parse(
  readFileSync('tools/license-exceptions.json', 'utf8'),
) as LicenseException[];
const allowed = new Set<string>([
  '0BSD',
  'Apache-2.0',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'CC0-1.0',
  'ISC',
  'MIT',
  'MPL-2.0',
  'Python-2.0',
  'Unicode-3.0',
  'Zlib',
]);
const failures: string[] = [];
const packages = readInstalledPackageJsons('node_modules');

for (const { path, metadata } of packages) {
  if (!metadata.name?.trim()) {
    continue;
  }

  const packageName = metadata.name;
  const license = normalizeLicense(metadata.license);
  if (!license) {
    if (!hasException(packageName, 'UNKNOWN')) {
      failures.push(`${path}: missing license metadata`);
    }
    continue;
  }

  if (!isLicenseExpressionAllowed(license) && !hasException(packageName, license)) {
    failures.push(`${path}: license ${license} is not in the allowlist or exception list`);
  }
}

if (failures.length > 0) {
  console.error('License policy failures:');
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log('Node dependency licenses satisfy the workspace policy.');

function readInstalledPackageJsons(root: string): { path: string; metadata: PackageJson }[] {
  const packagesRoot = join(root, '.pnpm');
  if (!existsSync(packagesRoot)) {
    return [];
  }

  const packageJsons: { path: string; metadata: PackageJson }[] = [];
  for (const entry of readdirSync(packagesRoot)) {
    const nodeModules = join(packagesRoot, entry, 'node_modules');
    if (!existsSync(nodeModules)) {
      continue;
    }

    for (const packageEntry of readdirSync(nodeModules)) {
      if (packageEntry.startsWith('@')) {
        for (const scopedEntry of readdirSync(join(nodeModules, packageEntry))) {
          pushPackageJson(
            packageJsons,
            join(nodeModules, packageEntry, scopedEntry, 'package.json'),
          );
        }
        continue;
      }

      pushPackageJson(packageJsons, join(nodeModules, packageEntry, 'package.json'));
    }
  }

  return packageJsons;
}

function pushPackageJson(
  packageJsons: { path: string; metadata: PackageJson }[],
  path: string,
): void {
  if (!existsSync(path)) {
    return;
  }

  packageJsons.push({
    path,
    metadata: JSON.parse(readFileSync(path, 'utf8')) as PackageJson,
  });
}

function normalizeLicense(license: LicenseMetadata | undefined): string | null {
  if (typeof license === 'string') {
    return license.trim();
  }

  if (license && typeof license.type === 'string') {
    return license.type.trim();
  }

  return null;
}

function isLicenseExpressionAllowed(expression: string): boolean {
  return expression
    .replace(/[()]/g, '')
    .split(/\s+OR\s+/u)
    .some((branch) =>
      branch
        .split(/\s+AND\s+/u)
        .map((license) => license.trim())
        .every((license) => allowed.has(license) && !isBlockedLicense(license)),
    );
}

function isBlockedLicense(license: string): boolean {
  return /^(A?GPL|LGPL)-/u.test(license);
}

function hasException(packageName: string, license: string): boolean {
  return exceptions.some(
    (exception) =>
      matchesPackagePattern(packageName, exception.package) &&
      (exception.license === license || exception.license === 'UNKNOWN'),
  );
}

function matchesPackagePattern(packageName: string, pattern: string): boolean {
  if (pattern.endsWith('*')) {
    return packageName.startsWith(pattern.slice(0, -1));
  }

  return packageName === pattern;
}
