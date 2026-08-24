# Contributing to asusctl

Thank you for contributing to asusctl. This guide outlines project bounds, issue reporting rules, and the pull request workflow.

## Project scope and support bounds

### Scope

The goal of this project is to bring full Linux support for every feature ASUS officially enables on the hardware it ships.

## Reporting issues

### Test on the latest version

Always test against the latest release or current `main` branch before submitting a bug report. Issues opened against older versions will be asked to retest.

### Follow provided templates

Issue templates have been added that will ask you to fill certain details based on the selected issue type. Please ensure all requested data is filled before submitting the issue.

### Use of LLMs

Using AI to diagnose a problem or pull information off your own device is fine and often useful. However, we strongly request that you do not hand us a report written entirely by AI. AI can hallucinate, and it dumps large amounts of verbose text that may or may not be correct, and can cost valuable time. If you think an AI diagnosis is worth including, include the findings, but clearly mention that it came from AI and we will weigh it in our own diagnosis from there.

We also reserve the right to remove any AI-generated comments that we deem do more harm to an issue's conversation history than good if needed to keep conversations cleaner.

## Pull request workflow

### Before starting

For significant changes, open an issue or discuss your proposed design in the Discord server first. This prevents duplicate effort and ensures compatibility with daemon components.

### What not to PR

There are certain things that we do not accept PRs for and should instead be suggested to us via an issue or in Discord and left to the maintainers to handle. This includes:
- Changes to CI/CD pipelines.
- Refactors that are not being discussed beforehand.
- Package version bumps and lockfile changes.
- AI work that you do not understand yourself.

### Commit standards

- Follow Conventional Commits format for commit titles (for example, `fix(rog-platform): resolve sysfs node parsing` or `refactor(asusctl): modularize cli handlers`).
- Do not use `--no-verify` or `-n` to bypass git hooks. All commits and pushes must run repository hooks normally.

### Verification suite

Run these checks before submitting your pull request:
- `cargo check --all-targets`
- `cargo test --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo cranky`
- `cargo fmt --all -- --check`

### Submitting a PR

- Fill out the pull request template completely, including verification steps and tested hardware details where applicable.
- Do not bump workspace or package versions in `Cargo.toml`. Version updates are managed by maintainers during releases.
- Link related issues using standard keywords such as `Fixes #123` or `Supersedes #456`.
- Ensure commits are separated by concern and each commit is minimal, doing one thing.
- Do not send PRs that you have not tested or cannot test yourself.
- If you are fixing an issue, mention what the issue is, how your PR fixes the issue, and how you have tested. The fix should be minimal and any cleaning up must be another commit after.
- Refactor commits must not cause any behaviour changes.
- The PR title and description must match the submitted code. A discrepancy can delay or prevent approval. If the discrepancy is intentional or materially misleading, maintainers may take further action under the “Breaking rules” section.

### Use of LLMs

We are not against using LLMs to improve productivity if you know what you are doing. However, if you are using purely AI with a blindfold on, this is not acceptable to us, and maintainers will deal with the latter as they see fit.

If you do want to use AI for a specific part of your work, you may simply ask us, with a justification as to why you need AI for it. We will then reply with a yes or no. The approval is valid for that part of the work and not for anything else, and you remain responsible for reviewing and understanding whatever you submit.

A list of possible actions we may take against such PRs is listed below.

## Breaking rules

If you break any of the rules listed above, the maintainers hold the right to exercise a sanction or punishment they deem fit for the case against the rule-breaking user. Below are a few examples of what may happen:
- You may be banned from further PR submission for a given time or forever.
- You may become ineligible to become a part of ASUS Linux.
- If you are a member of OGC and/or ASUS Linux team, your membership may be revoked or you may be placed back in probation.
- You may be muted/banned from our Discord.

This is not a comprehensive list of actions that may be taken, and the action will be decided by the maintainers.

## Code of conduct

### Our pledge

We pledge to make participation in our project a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, gender identity and expression, level of experience, nationality, personal appearance, race, religion, or sexual identity.

### Standards

Positive behaviors include using inclusive language, respecting differing viewpoints, accepting constructive criticism, and focusing on community well-being.

Unacceptable behaviors include sexualized language or imagery, trolling, derogatory comments, personal attacks, public or private harassment, and publishing private information without explicit consent.

### Enforcement

Maintainers enforce these standards and may remove comments, reject code, or ban contributors who violate them. Report violations to project maintainers or contact `benato.denis96@gmail.com`. All complaints are reviewed confidentially.
