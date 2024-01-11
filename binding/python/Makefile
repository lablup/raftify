build:
	maturin build

build-release:
	maturin build --release --strip

install:
	maturin build
	pip install .

uninstall:
	pip uninstall -y raftify_cli

install-cli:
	cd examples/cli
	pip install .
	cd ../..

reinstall-cli:
	pip uninstall -y raftify_cli
	cd examples/cli
	pip install .
	cd ../..

install-release:
	make build-release
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
	make build-release
	maturin publish

test:
	cd tests && make test && cd ..
