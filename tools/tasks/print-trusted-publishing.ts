const packages: string[] = [
  '@oxlint-plugins/oxlint-plugin-type-aware',
  '@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers',
];

for (const pkg of packages) {
  console.log(
    `npm trust github ${pkg} --repo ubugeeei-prod/oxlint-plugins --file release.yml --env npm-publish --allow-publish`,
  );
}
