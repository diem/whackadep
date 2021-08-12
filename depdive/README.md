# Depdive

[![depdive on crates.io](https://img.shields.io/crates/v/depdive)](https://crates.io/crates/depdive)
[![Documentation (latest release)](https://docs.rs/depdive/badge.svg)](https://docs.rs/depdive/)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](LICENSE-APACHE)

Depdive is a Rust dependency analysis tool,
that provides various analysis metrics for
i) Rust crates to aid in dependency selection and monitoring,
i) and their version updates, to aid in security review
(e.g., for pull requests created by dependabot).


# Installation

1. You can use depdive as a Rust crate in your project that provides individual access to all the analysis offered.
2. You can also use the CLI tool through `cargo install depdive`.
3. For dependency update review, depdive outputs a markdown formatted string. Therefore, you can integrate depdive into your CI tooling to automatically get depdive comments on PR that updates a dependency. [See this example](https://github.com/diem/diem/blob/main/.github/workflows/dep-update-review.yml).

# Usage

1. **Dependency update review**: You can provide two commits for a given repo, or two paths for a repo checked out at two different commits in order to compare the dependencies that have been upgraded between the two commits and get depdive review report for those updates in markdown format. Check functions `run_update_analyzer_from_repo_commits` and `run_update_analyzer_from_paths` at the library root.
When used as a CLI tool, you can run `depdive update-review commits <repo-path> <commit_a> <commit_b>` or `depdive update-review paths <path_a> <path_b>`.

2. **Dependency monitoring metrics**: You can provide the path of your Cargo project and get the dependency monitoring metrics in `json` format. Check impls of `DependencyAnalyzer` and `DependencyGraphAnalyzer` at the library root.
When used as a CLI tool, you can run `depdive dep-review package-metrics <path>` and `depdive dep-review code-metrics <path>` to get usage and activity metrics and code and unsafe analysis metrics respectively. Note that, code-mterics use [`cargo-geiger`](https://github.com/rust-secure-code/cargo-geiger) which cannot be run more than once at a time.


## Dependency Update Review

Depdive offers below analysis for a Rust dependency update:

1. Presence of known advisories
2. Change in build script files
3. Change in unsafe files
4. If code hosted on crates.io differs from the git source
5. Version diff summary, list of changed files.

The markdown comment looks like this with i) a table with checkboxes for four criteria, and ii) details available on a click.
![image](https://user-images.githubusercontent.com/31052507/128957013-6dc01a2b-6a13-4692-8c0c-c6951c92e4f3.png)


## Dependency Monitoring Metrics

Depdive offers below analysis for dependency selection/monitoring:

1. **Usage metrics**: Crates.io downloads, dependents; GitHub stars, subscribers, forks.
2. **Activity metrics**: Days since last commit, last opened issue; no. of commits, issues in last six months; no. of open issues with `bug`, `security` label.
3. **Code analysis**: Total lines of code (LOC), total LOC pulled in through its own deps; Total LOC pulled in through exclusive deps - deps only introduced transitively by this one; if the crate has build script; how many of its deps have build script.
4. **Unsafe analysis**: Depdive uses [`cargo-geiger`](https://github.com/rust-secure-code/cargo-geiger) to provide count of unsafe code in a Rust crate, and also total unsafe code pulled in by a crate through its dependencies.

# Why care about security reviewing dependency updates?

You are essentially pulling in new code to your codebase each time you make a dependency update and thus, creating a channel for security holes to sneak in. While manually reviewing dependency updates, there can be some routine checks that can be automated. The goal of depdive is to aid you in dep update review by performing such automated checks. Please, let us know what other analysis you think can be helpful.

# Known issues

1. Dependency update review shows if there is a change in the build script during an update. However, we only check if the crate contains a build script and the path to that build script is present in the version diff. However, the build script can call external modules and execute external code by such ways. However, we do not check if any external code potentially executed by the build script is modified or not.
