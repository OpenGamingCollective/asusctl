# Contributing to asusctl

Thank you for contributing to asusctl. This guide outlines project bounds, issue reporting rules, and the pull request workflow.

## Project scope and support bounds

### Scope
The goal of this project is making ASUS hardware usable under Linux.

### Supported distributions
Arch Linux and Arch-based distributions are officially supported. Issues specific to unsupported distributions like Ubuntu may be closed without investigation due to maintainer time constraints.

## Reporting issues

### Test on the latest version
Always test against the latest release or current `main` branch before submitting a bug report. Issues opened against older versions will be asked to retest.

### Include hardware context
Include your laptop model, DMI board name, kernel version, distribution name, and exact asusctl version in your report.

## Pull request workflow

### Before starting
For significant changes, open an issue or discuss your proposed design in the Discord server first. This prevents duplicate effort and ensures compatibility with daemon components.

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
- Fill out the pull request template completely, including tested hardware details and verification steps.
- Do not bump workspace or package versions in `Cargo.toml`. Version updates are managed by maintainers during releases.
- Link related issues using standard keywords such as `Fixes #123` or `Supersedes #456`.

## Code of conduct

### Our pledge
We pledge to make participation in our project a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, gender identity and expression, level of experience, nationality, personal appearance, race, religion, or sexual identity.

### Standards
Positive behaviors include using inclusive language, respecting differing viewpoints, accepting constructive criticism, and focusing on community well-being.

Unacceptable behaviors include sexualized language or imagery, trolling, derogatory comments, personal attacks, public or private harassment, and publishing private information without explicit consent.

### Enforcement
Maintainers enforce these standards and may remove comments, reject code, or ban contributors who violate them. Report violations to project maintainers or contact `luke@ljones.dev`. All complaints are reviewed confidentially.