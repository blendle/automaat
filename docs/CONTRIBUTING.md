# Contributing to Automaat

Automaat welcomes contribution from everyone in the form of suggestions, bug
reports, pull requests, and feedback. This document gives some guidance if you
are thinking of helping us.

Please reach out here in a GitHub issue if we can do anything to help you
contribute.

## Submitting bug reports and feature requests

Automaat development is spread across multiple crates, but all active
development happens in this repository and can be used for opening any issues
related to Automaat.

When reporting a bug or asking for help, please include enough details so that
the people helping you can reproduce the behavior you are seeing. For some tips
on how to approach this, read about how to produce a [Minimal, Complete, and
Verifiable example].

[minimal, complete, and verifiable example]: https://stackoverflow.com/help/mcve

When making a feature request, please make it clear what problem you intend to
solve with the feature, any ideas for how Automaat could support solving that
problem, any possible alternatives, and any disadvantages.

## Useful resources

Here's a list of useful resources that can help you during your contributions to
the project:

- [Rust Language Cheat Sheet](https://cheats.rs)
- [Rust API Guidelines](https://rust-lang-nursery.github.io/api-guidelines/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust By Example](https://doc.rust-lang.org/stable/rust-by-example/)
- [Rust Reference](https://doc.rust-lang.org/stable/reference/)
- [Read Rust](https://readrust.net)
- [Rust Standard Library](https://doc.rust-lang.org/stable/std/)
- [Rust Compiler Error Index](https://doc.rust-lang.org/error-index.html)
- [Rust Learning Resources](https://github.com/ctjhoa/rust-learning)

## Running the test suite

We encourage you to check that the test suite passes locally before submitting a
pull request with your changes. If anything does not pass, typically it will be
easier to iterate and fix it locally than waiting for the CI servers to run
tests for you.

##### In the root directory

```sh
# Test all the crates in the workspace
cargo test --all
```

## Conduct

In all Automaat-related communication, we follow the [Rust Code of Conduct]. For
escalation or moderation issues please contact Jean <jean@blendle.com> instead
of the Rust moderation team.

[rust code of conduct]: https://www.rust-lang.org/conduct.html

## Acknowledgements

_the contents of this document were kindly borrowed from `serde-rs/serde`_
