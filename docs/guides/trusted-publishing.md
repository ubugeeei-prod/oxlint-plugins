# Trusted Publishing

This repository assumes npm trusted publishing. Do not create or store an npm publish token for release automation.

Configure every public package once after it exists on npm:

```sh
vp run trusted-publishing
```

The command prints the `npm trust github ...` commands for every package.

The release workflow intentionally uses two runner families:

- Blacksmith `blacksmith-32vcpu-ubuntu-2404` for CI verification.
- GitHub-hosted `ubuntu-latest` only for the final publish job, because npm trusted publishing currently supports GitHub Actions only on GitHub-hosted runners.

The release verification job runs `pnpm run verify`. The publish job rebuilds package artifacts on the trusted publishing runner, checks package dry-run output, and publishes with provenance.

The trusted publisher should be configured for:

- Repository: `ubugeeei-prod/oxlint-plugins`
- Workflow file: `release.yml`
- Environment: `npm-publish`
- Allowed action: `pnpm publish --recursive --access public --provenance --no-git-checks`

For maximum release control, configure the `npm-publish` GitHub environment with required reviewers and protect release tags.
