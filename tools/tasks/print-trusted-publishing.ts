const packages: string[] = [
  '@oxlint-plugins/oxlint-plugin-type-aware',
  '@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers',
  '@oxlint-plugins/oxlint-plugin-functional',
  '@oxlint-plugins/oxlint-plugin-stylistic',
];

for (const pkg of packages) {
  console.log(
    `npm trust github ${pkg} --repo ubugeeei-prod/oxlint-plugins --file release.yml --env npm-publish --allow-publish`,
  );
}
