# OpenAI Rust Library Contributing Guide

Thank you for your interest in contributing to OpenAI Rust Library! We welcome all contributions, whether it's reporting an issue, fixing a bug, or implementing a new feature. Before you get started, please take a moment to review the following guidelines.

## Pull Request Guidelines

- Checkout a topic branch from a base branch, e.g. `master`, and merge back against that branch.

- If adding a new feature:

  - Add accompanying test case.
  - Provide a convincing reason to add this feature. Ideally, you should open a suggestion issue first and have it approved before working on it.

- If fixing bug:

  - If you are resolving a special issue, add `(fix #xxxx[,#xxxx])` (#xxxx is the issue id) in your PR title for a better release log, e.g. `fix: update entities encoding/decoding (fix #3899)`.
  - Provide a detailed description of the bug in the PR. Live demo preferred.
  - Add appropriate test coverage if applicable.

- It's OK to have multiple small commits as you work on the PR - GitHub can automatically squash them before merging.

- Make sure tests pass!

- Commit messages must follow the [Angular Team's Commit Message Guidelines](https://github.com/angular/angular/blob/master/CONTRIBUTING.md#commit) so that changelogs can be automatically generated. Commit messages are automatically validated before commit.
- No need to worry about code style as long as you have installed the dev dependencies - modified files are automatically formatted with Prettier on commit.
