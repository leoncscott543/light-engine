# Development container for the Rust alpha
FROM ubuntu:20.04

LABEL Name="light-engine" Version="0.1.0"

ENV DEBIAN_FRONTEND=noninteractive

# --- Set the locale to avoid issues with non-UTF-8 characters ---
RUN apt-get update && apt-get install -y locales && \
    locale-gen en_US.UTF-8 && \
    update-locale LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

# --- Basic build tools ---
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    pkg-config \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# --- Install Rust toolchain (with Cargo and utilities) ---
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app
COPY . /app
