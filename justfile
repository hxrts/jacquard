default:
    @just --list

# check workspace compiles
check:
    cargo check --workspace

# build all crates
build:
    cargo build --workspace

# run all tests
test:
    cargo test --workspace

# run clippy lints
lint:
    cargo clippy --workspace -- -D warnings

# format code
fmt:
    cargo fmt --all

# check formatting
fmt-check:
    cargo fmt --all -- --check

# install git hooks
install-hooks:
    git config core.hooksPath .githooks
    @echo "Git hooks installed. Pre-commit checks will run automatically."
