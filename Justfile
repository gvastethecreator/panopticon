# Justfile for Panopticon
# Run tasks with: just <task>

# 🔨 Build (debug)
build:
    cargo build --locked

# 🚀 Build (release, optimized)
release:
    cargo build --release --locked

# ✅ Type-check without building
check:
    cargo check --locked

# 🧹 Lint with Clippy (pedantic, deny warnings, all targets)
lint:
    cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic

# 🎨 Format all source files
fmt:
    cargo fmt

# 🎨 Check formatting (CI-friendly)
fmt-check:
    cargo fmt -- --check

# 🧪 Run all tests
test:
    cargo test --all-targets --locked

# 📊 Generate coverage report (requires cargo-tarpaulin)
coverage:
    cargo tarpaulin --out html --output-dir target/coverage

# 📖 Build and open rustdoc documentation
doc:
    cargo doc --no-deps --locked

doc-open:
    cargo doc --no-deps --locked --open

# Requires: cargo install cargo-audit --locked
audit:
    cargo audit

# 🏃 Run (debug)
run:
    cargo run --locked

# 🏃 Run (release)
run-release:
    cargo run --release --locked

# 🧼 Remove build artifacts
clean:
    cargo clean

# 🔄 Full CI pipeline
ci: check fmt-check lint test release doc audit
