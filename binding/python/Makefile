build:
	maturin build

install:
	maturin build
	pip install .

install-release:
	maturin build --release --strip
	pip install .

clean:
	cargo clean

test:
	python -m pytest

fmt:
	cargo fmt

lint:
	cargo clippy 

lint-fix:
	cargo clippy --fix --lib -p bindings
	cargo clippy --fix --lib -p external-bindings
	cargo clippy --fix --lib -p mem-storage-bindings
	cargo clippy --fix --lib -p py-storage-bindings
	cargo clippy --fix --lib -p raftpb-bindings
	cargo clippy --fix --lib -p utils

lint-fix-py:
	python -m black **/*.py

publish:
	maturin build --release --strip
	maturin publish
