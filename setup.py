import os
from pathlib import Path

from setuptools import setup

ROOT_PATH = os.path.abspath(os.path.dirname(__file__))
VERSION = (Path(__file__).parent / "raftify" / "VERSION").read_text().strip()
README = (Path(__file__).parent / "README.md").read_text().strip()


def get_requirements(env: str = None):
    requirements = "requirements"
    if env is not None:
        requirements = f"{requirements}-{env}"
    with open(f"{requirements}.txt".format(env)) as fp:
        return [x.strip() for x in fp.read().split("\n") if not x.startswith("#")]


default_requires = get_requirements()
dev_requires = get_requirements("dev")
build_requires = get_requirements("build")


setup(
    name="raftify",
    version=VERSION,
    long_description=README,
    long_description_content_type='text/markdown',
    author="Lablup Inc.",
    maintainer="jopemachine",
    maintainer_email="gbl@lablup.com",
    url="https://github.com/lablup/raftify",
    license="Apache License 2.0",
    package_dir={"raftify": "raftify"},
    package_data={"": ["VERSION"]},
    python_requires=">=3.10",
    install_requires=default_requires,
    extras_require={"dev": dev_requires, "build": build_requires},
    zip_safe=False,
    include_package_data=True,
)
