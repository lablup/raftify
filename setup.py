import os
from pathlib import Path

from setuptools import setup

ROOT_PATH = os.path.abspath(os.path.dirname(__file__))
VERSION = (Path(__file__).parent / "riteraft" / "VERSION").read_text().strip()
description = (
    "A raft framework, for regular people. Written in Python."
)


def get_requirements(env: str = None):
    requirements = "requirements"
    if env is not None:
        requirements = f"{requirements}-{env}"
    with open(f"{requirements}.txt".format(env)) as fp:
        return [x.strip() for x in fp.read().split("\n") if not x.startswith("#")]


install_requires = get_requirements()
dev_requires = get_requirements("dev")


setup(
    name="riteraft",
    version=VERSION,
    description=description,
    author="Lablup Inc.",
    maintainer="jopemachine",
    maintainer_email="gbl@lablup.com",
    url="https://github.com/lablup/riteraft-py",
    license="Apache License 2.0",
    package_dir={"riteraft": "riteraft"},
    package_data={"": ["VERSION"]},
    python_requires=">=3.10",
    install_requires=install_requires,
    extras_require={"dev": dev_requires},
    zip_safe=False,
    include_package_data=True,
)
