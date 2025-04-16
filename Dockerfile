# Start with a minimal Ubuntu base image
FROM ubuntu:20.04

LABEL Name="Light Engine" Version="0.1.0" Description="Light Engine simulation, AI, rendering, and cloud support"

# Set non-interactive mode for apt-get to avoid prompts during installation
ENV DEBIAN_FRONTEND=noninteractive

# Install essential build dependencies and tools
RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    curl \
    cmake \
    wget \
    unzip \
    python3-pip \
    libxcb1-dev \
    libx11-dev \
    libxcb-xfixes0-dev \
    libxcb-randr0-dev \
    libxcb-shape0-dev \
    libxcb-sync-dev \
    libx11-xcb-dev \
    libxrandr-dev \
    xcb-proto \
    libxfixes-dev \
    libxcomposite-dev \
    libxdamage-dev \
    clang \
    llvm \
    lldb \
    libclang-dev \
    libssl-dev \
    zlib1g-dev \
    libcurl4-openssl-dev \
    libjsoncpp-dev \
    libfreetype6-dev \
    libfontconfig1-dev \
    libglu1-mesa-dev \
    libtiff-dev \
    libopenal-dev \
    libsndfile1-dev \
    pkg-config \
    libprotobuf-dev \
    protobuf-compiler \
    libjemalloc-dev \
    libsqlite3-dev \
    libsdl2-dev \
    libjpeg-dev \
    libpng-dev \
    libassimp-dev \
    libtinyxml-dev \
    lsb-release \
    sudo \
    gnupg2 \
    apt-transport-https \
    clang-tools \
    libncurses5-dev \
    libwebp-dev

# Only install multilib and i386 dev packages on amd64
RUN if [ "$(dpkg --print-architecture)" = "amd64" ]; then \
      apt-get update && apt-get install -y gcc-multilib g++-multilib libc6-dev-i386; \
    fi

# Install Vulkan SDK
RUN apt-get update && apt-get install -y libvulkan-dev vulkan-utils

# Install Rust toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set the working directory in the container
WORKDIR /app

# Copy your project files into the container
COPY . /app

# Expose necessary ports (e.g., for cloud services, simulators)
EXPOSE 8080

# Default command (override this as necessary)
CMD ["bash"]
