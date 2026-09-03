# Releasing Wave

Wave releases are created by the manual `Manual Release` GitHub Actions workflow.
The workflow validates the compiler before it creates any tag or GitHub release.

Wave uses release-count versioning by development stage. Tags keep the
`vX.Y.Z-stage` shape, but major/minor/patch positions do not carry SemVer
meaning. Every public release advances the stage counter by one step.

Regular releases are scheduled for the 5th day of every even-numbered month:
February, April, June, August, October, and December.

## Release prerequisites

1. Merge the release candidate into `master`.
2. Remove the development `-dev` suffix in `Cargo.toml` and update `Cargo.lock`.
3. Confirm that the normal `Wave CI` workflow passes on `master`.
4. Prepare and review the release post separately in the Wave blog repository.

## Run the release

1. Open the repository's **Actions** page.
2. Select **Manual Release**.
3. Select **Run workflow** and choose the `master` branch.
4. Enter the version without a `v` prefix, such as `0.2.0-pre-beta`.
5. Keep **draft** and **prerelease** enabled for a pre-beta release.

The workflow rejects non-`master` revisions, malformed versions, a version that
does not match `Cargo.toml`, and an existing release tag. It then runs formatting,
Clippy, Rust tests, a release build, compiler version checks, and the Wave
end-to-end test suite.

After validation, separate jobs package and smoke-test these toolchains:

- `x86_64-linux-gnu`
- native macOS (`aarch64-apple-darwin` or `x86_64-apple-darwin`)
- `x86_64-pc-windows-gnu`

The final job verifies every archive checksum and creates the GitHub release.
No release is created if validation, packaging, or a smoke test fails. With the
default inputs, the result remains a draft until a maintainer reviews and
publishes it from GitHub.

## Recovery

Rerun a failed workflow only after fixing the cause on `master`. If the final
release creation failed after a tag was created, inspect and remove or retain the
partial GitHub release deliberately before retrying; do not overwrite a release
tag automatically.
