# @oxlint-plugins/oxlint-plugin-astro

Rust-backed Oxlint plugin port of selected `eslint-plugin-astro` rules.

This first slice implements three deprecated-API rules by extracting Astro
frontmatter and parsing it as TypeScript with Oxc:

- `no-deprecated-astro-canonicalurl`
- `no-deprecated-astro-fetchcontent`
- `no-deprecated-getentrybyslug`

Template expressions are intentionally not scanned in this slice. React rules
are outside the Astro port's scope because Oxc handles React rules separately.
