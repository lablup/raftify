import os
from pathlib import Path

from setuptools import setup

ROOT_PATH = os.path.abspath(os.path.dirname(__file__))
VERSION = (Path(__file__).parent / "VERSION").read_text().strip()
README = (Path(__file__).parent / "README.md").read_text().strip()


def get_requirements(env: str = None):
    requirements = "requirements"
    if env is not None:
        requirements = f"{requirements}-{env}"
    with open(f"{requirements}.txt".format(env)) as fp:
        return [x.strip() for x in fp.read().split("\n") if not x.startswith("#")]


default_requires = get_requirements()

setup(
    name="raftify_cli",
    version=VERSION,
    description="Experimental Async Raft framework for Python application",
    long_description=README,
    long_description_content_type='text/markdown',
    author="Lablup Inc.",
    maintainer="jopemachine",
    maintainer_email="jopemachine@naver.com",
    url="https://github.com/lablup/raftify",
    license="Apache License 2.0",
    package_dir={"raftify_cli": "raftify_cli"},
    package_data={"": ["VERSION"]},
    python_requires=">=3.10",
    install_requires=default_requires,
    zip_safe=False,
    include_package_data=True,
    entry_points={
        'console_scripts': [
            'raftify_cli = raftify_cli.cli:main'
        ]
    }
)
