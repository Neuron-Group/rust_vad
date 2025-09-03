FROM nvidia/cuda:12.2.0-devel-ubuntu22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/root/.cargo/bin:${PATH}"

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*
    
RUN apt-get update && apt-get install -y libasound2-dev
RUN apt-get install git -y
RUN apt-get install libclang-dev -y
RUN apt-get install pkg-config -y
RUN apt-get install cmake -y

# 仅安装 nightly Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --default-toolchain nightly \
    --profile minimal

RUN rustup component add rust-src
RUN rustup component add rustfmt

RUN rustc --version && cargo --version && nvcc --version

WORKDIR /workspace
CMD ["bash"]
