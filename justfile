default:
  @just --list

setup_lefthook:
  @lefthook install

setup_environment:
  @cargo install cargo-nextest

# Setup the whole project.
setup: setup_lefthook

# Run all pre-commit checks
pre-commit: fmt_check clippy test
  @echo "All pre-commit checks passed!"

# Format all code
fmt:
  cargo fmt --all

# Check formatting without modifying
fmt_check:
  cargo fmt --all -- --check

# Run clippy with warnings as errors
clippy:
  cargo clippy --all --all-targets -- -D warnings

# Run all tests
test:
  cargo test --all

# Build all targets
build:
  cargo build --all

# Clean build artifacts
clean:
  cargo clean

# Run all cargo benchmarks
bench:
  cargo bench --all

# Run full CI simulation locally
ci: fmt_check clippy test
  @echo "Full CI simulation passed!"

# Install git hooks via lefthook
install_hooks:
  lefthook install

# === Version Management (cargo-workspaces) ===

# Bump patch version (0.1.0 -> 0.1.1) for all crates
release_patch:
  cargo ws version patch --no-git-push --yes

# Bump minor version (0.1.0 -> 0.2.0) for all crates
release_minor:
  cargo ws version minor --no-git-push --yes

# Bump major version (0.1.0 -> 1.0.0) for all crates
release_major:
  cargo ws version major --no-git-push --yes

# Set exact version for a crate: just set_version eazy 0.2.0
set_version crate version:
  cargo set-version -p {{crate}} {{version}}

# Bump all eazy-* crates together
bump_eazy bump:
  cargo set-version -p eazy-core --bump {{bump}}
  cargo set-version -p eazy-derive --bump {{bump}}
  cargo set-version -p eazy-tweener --bump {{bump}}
  cargo set-version -p eazy-keyframe --bump {{bump}}
  cargo set-version -p eazy --bump {{bump}}

# Bump all zo-* crates together
bump_codelord bump:
  #!/usr/bin/env sh
  for crate in $(cargo ws list | grep '^codelord-'); do
    cargo set-version -p "$crate" --bump {{bump}}
  done
  cargo set-version -p codelord --bump {{bump}}

# List all workspace crates and their versions
list_versions:
  cargo ws list -l

# Show what would change without applying
release_dry_run bump="patch":
  cargo ws version {{bump}} --no-git-push --dry-run

# Publish a single crate: just publish eazy
publish crate:
  cargo publish -p {{crate}}

# Dry-run publish (verify without uploading)
publish_dry crate:
  cargo publish -p {{crate}} --dry-run

# Create a release tag: just release 0.0.0
release version:
  git tag -a {{version}} -m "codelord {{version}}"
  git push origin {{version}}

# Delete a tag (if you made a mistake): just delete_tag 0.1.0
delete_tag version:
  git tag -d {{version}}
  git push origin :refs/tags/{{version}}

# List all tags
list_tags:
  git tag -l --sort=-v:refname
