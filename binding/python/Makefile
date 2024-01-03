build:
	maturin build

install:
	maturin build
	pip install .

install-cli:
	cd cli
	pip install .
	cd ..

install-release:
	maturin build --release --strip
	pip install .

clean:
	cargo clean

fmt:
	cargo fmt

lint:
	cargo clippy 

lint-fix-py:
	python -m black raftify.pyi
	python -m black **/*.py

publish:
	maturin build --release --strip
	maturin publish
