# Wave Versioning Policy

Wave uses release-count versioning by development stage.

The tag format still uses a familiar shape such as `v0.2.0-pre-beta`, but Wave
does not use SemVer meanings for major, minor, and patch numbers. A release is a
release: when a new public release is published, the version advances by one
step, regardless of whether the change is small, large, internal, or
user-visible.

## Release Cadence

Wave uses a regular bimonthly release cycle so maintainers can work sustainably
and users can keep a predictable development environment.

Regular releases are published on the 5th day of every even-numbered month:

- February 5
- April 5
- June 5
- August 5
- October 5
- December 5

Emergency fixes may be released separately when maintainers decide that waiting
for the next regular release would be harmful.

## Development Stages

- `pre-alpha`: very early compiler work.
- `pre-beta`: frontend, CLI, LLVM backend, standard library, and release
  packaging are stabilized.
- `alpha`: Whale, Wave's own toolchain intended to coexist with and eventually
  replace LLVM-based paths, is developed and optimized.
- `beta`: the language and toolchain are stable enough for broader external
  testing.
- `rc`: release-candidate builds for final stabilization.
- stable release: general-use releases.

When Wave moves to a new stage, the release counter resets for that stage. For
example, after the final `pre-beta` release, the first `alpha` release starts at
`v0.0.1-alpha`.

## Current Sequence

`v0.1.9-pre-beta` is the 19th `pre-beta` release.

The next `pre-beta` release is `v0.2.0-pre-beta`.

## Rules

- The version in `Cargo.toml` must match the release workflow input.
- Git tags use the `v` prefix, for example `v0.2.0-pre-beta`.
- Release workflow input omits the `v` prefix, for example `0.2.0-pre-beta`.
- Do not describe Wave releases as SemVer-compatible unless the policy changes.
- Do not assign compatibility promises to the major, minor, or patch positions.
