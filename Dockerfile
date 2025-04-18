# Start with a minimal Ubuntu base image
FROM ubuntu:20.04

LABEL Name="Light Engine" Version="0.1.0" Description="Light Engine simulation, AI, rendering, and cloud support"

ENV DEBIAN_FRONTEND=noninteractive

# --- Set the locale to avoid issues with non-UTF-8 characters ---
RUN apt-get update && apt-get install -y locales && \
    locale-gen en_US.UTF-8 && \
    update-locale LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

# --- Install latest Git from source ---
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    libcurl4-openssl-dev \
    libexpat1-dev \
    gettext \
    unzip \
    wget \
    zlib1g-dev \
    && apt-get clean && rm -rf /var/lib/apt/lists/*
RUN wget https://github.com/git/git/archive/refs/tags/v2.44.0.zip -O /tmp/git.zip && \
    unzip /tmp/git.zip -d /tmp && \
    cd /tmp/git-2.44.0 && \
    make prefix=/usr/local all && \
    make prefix=/usr/local install && \
    cd / && rm -rf /tmp/git*

# --- Install Rust toolchain (with Cargo and utilities) ---
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# --- Core cross-platform build tools, Python, and utilities ---
RUN apt-get update && apt-get install -y \
    cmake \
    python3 \
    python3-pip \
    sudo \
    pkg-config \
    clang \
    llvm \
    lldb \
    libclang-dev \
    clang-tools \
    curl \     
    # --- Vulkan (cross-platform API, system libs for Linux) ---
    libvulkan-dev \
    vulkan-utils \
    # --- Protobuf (for ML/data, cross-platform) ---
    libprotobuf-dev \
    protobuf-compiler \
    # --- SQLite (cross-platform DB) ---
    libsqlite3-dev \
    # --- Compression and crypto (cross-platform) ---
    zlib1g-dev \
    libssl-dev \
    ca-certificates \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# --- Install kubectl (Kubernetes CLI, optional, cross-platform tool) ---
RUN curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl" \
    && install -m 0755 kubectl /usr/local/bin/kubectl \
    && rm kubectl

# --- Only install multilib and i386 dev packages on amd64 (optional, for cross-compiling) ---
RUN if [ "$(dpkg --print-architecture)" = "amd64" ]; then \
      apt-get update && apt-get install -y gcc-multilib g++-multilib libc6-dev-i386 && \
      apt-get clean && rm -rf /var/lib/apt/lists/*; \
    fi

# --- (Optional) Install Python ML libraries if you need them for FFI/wrappers ---
RUN pip3 install --no-cache-dir tensorflow torch torchvision torchaudio

WORKDIR /app
COPY . /app

EXPOSE 8080

CMD ["bash"]
