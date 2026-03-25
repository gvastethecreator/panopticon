# Justfile for Panopticon
# Run tasks with: just <task>

# 🔨 Build (debug)
build:
    cargo build

# 🚀 Build (release, optimised)
release:
    cargo build --release

# ✅ Type-check without building
check:
    cargo check

# 🧹 Lint with Clippy (pedantic, deny warnings)
lint:
    cargo clippy -- -D warnings -W clippy::pedantic

# 🎨 Format all source files
fmt:
    cargo fmt

# 🎨 Check formatting (CI-friendly)
fmt-check:
    cargo fmt -- --check

# 🧪 Run all tests
test:
    cargo test

# 📊 Generate coverage report (requires cargo-tarpaulin)
coverage:
    cargo tarpaulin --out html --output-dir target/coverage

# 📖 Build and open rustdoc documentation
doc:
    cargo doc --no-deps --open

# 🏃 Run (debug)
run:
    cargo run

# 🏃 Run (release)
run-release:
    cargo run --release

# 🧼 Remove build artefacts
clean:
    cargo clean

# 🔄 Full CI pipeline: format check → lint → test
ci: fmt-check lint test
