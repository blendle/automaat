# How to release new crate versions

## Prerequisites

It is generally best to start with a clean repository dedicated to a release so
that no git weirdness happens:

```shell
git clone https://github.com/blendle/automaat automaat_release
cd automaat_release
```

We use the `nightly` toolchain when releasing. This is because some of our
crates require nightly:

```shell
rustup default nightly
```

We use [`cargo-release`](cargo-release) to automate crate releases. You will
need to install it locally:

- `cargo install -f cargo-release`

## Preparing for a release

**All release commands must be run from the root directory of the repository.**

### Determine new release level

For the crate you are about to release, determine the desired release level
(`patch`, `minor`, `major`). Set the `RELEASE_LEVEL` environment variable to
the desired release level.

### Local test

Make sure there are no errors in the code by running the test suite:

```shell
cargo fmt -- --check
cargo build --all --all-features
cargo test --all --all-features
cargo doc --all --all-features --no-deps
```

### Dry run

It is a good idea to do a dry run to sanity check what actions will be
performed.

```shell
cargo release "$RELEASE_LEVEL" --manifest-path src/path/to/Cargo.toml --dry-run
```

## Release

After running the test suite, and validated the expected outcome via a dry run,
it is time to release. A release consists of bumping the crate version,
creating a new Git tag, and pushing the release to [crates.io].

This will all be handled by running the release command:

```shell
cargo release "$RELEASE_LEVEL" --manifest-path src/path/to/Cargo.toml
```

Once the command exits successfully, the new release is ready to be used.

For now, there is no CHANGELOG that needs to be updated, one will be created in
the future, and updated automatically using [_conventional commits_] and [_keep a
changelog_].

---

_some contents in this document are kindly borrowed from
`graphql-rust/juniper`_

[cargo-release]: https://github.com/sunng87/cargo-release
[crates.io]: https://crates.io
[_conventional commits_]: https://www.conventionalcommits.org
[_keep a changelog_]: https://keepachangelog.com
