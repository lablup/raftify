build:
	maturin build

build-release:
	maturin build --release --strip

install:
	maturin build
	pip install . --break-system-packages

uninstall:
	pip uninstall -y raftify_cli --break-system-packages

install-cli:
	cd examples/cli
	pip install . --break-system-packages
	cd ../..

reinstall-cli:
	pip uninstall -y raftify_cli --break-system-packages
	cd examples/cli
	pip install . --break-system-packages
	cd ../..

install-release:
	make build-release
	pip install . --break-system-packages

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
	maturin publish

test:
	cd tests && make test && cd ..
