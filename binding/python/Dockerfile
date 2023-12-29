FROM python:3.11
ARG branch=main

RUN pip install --upgrade pip \
    pip install -U -q pip setuptools

RUN mkdir /raftify
COPY . /raftify
WORKDIR /raftify

RUN apt-get update && \
    apt install protobuf-compiler tmux -y
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

RUN /bin/bash -c \
    "source $HOME/.cargo/env \
    && pip install .
