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
    libvulkan-dev \
    vulkan-utils \
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
    # Only install multilib and i386 dev packages on amd64
    && if [ "$(dpkg --print-architecture)" = "amd64" ]; then \
        apt-get install -y gcc-multilib g++-multilib libc6-dev-i386; \
    fi

# Install Rust and configure it (includes cargo and rustc)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Install Node.js and npm (needed for web-based tools or mobile dev)
RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash - && apt-get install -y nodejs

# Install dependencies for cross-compiling to macOS and mobile platforms
RUN apt-get install -y \
    clang \
    libobjc-7-dev \
    libstdc++-8-dev \
    libgcc-8-dev \
    crossbuild-essential-arm64 \
    crossbuild-essential-armhf \
    gcc-arm-none-eabi \
    libncurses-dev \
    libxml2-dev \
    libusb-dev \
    libssl-dev

# Install Vulkan SDK for rendering
RUN wget https://sdk.lunarg.com/sdk/download/latest/linux/vulkan-sdk.tar.xz -O /tmp/vulkan-sdk.tar.xz \
    && mkdir /tmp/vulkan-sdk \
    && tar -xvf /tmp/vulkan-sdk.tar.xz -C /tmp/vulkan-sdk \
    && cp -r /tmp/vulkan-sdk/vulkan-sdk/* /usr/local/

# Set environment variables for Vulkan SDK
ENV VULKAN_SDK=/usr/local/vulkan-sdk/x.x.x.x/x86_64
ENV PATH=$VULKAN_SDK/bin:$PATH
ENV LD_LIBRARY_PATH=$VULKAN_SDK/lib:$LD_LIBRARY_PATH
ENV VK_ICD_FILENAMES=$VULKAN_SDK/etc/vulkan/icd.d/nvidia_icd.json
ENV VK_LAYER_PATH=$VULKAN_SDK/etc/vulkan/explicit_layer.d

# Install WGPU dependencies for Rust-based WebGPU support
RUN apt-get install -y libwebp-dev libopencl-dev

# Install Docker CLI and Kubernetes tools for cloud integration
RUN curl -fsSL https://get.docker.com -o get-docker.sh \
    && sh get-docker.sh \
    && rm get-docker.sh

RUN curl -LO https://storage.googleapis.com/kubernetes-release/release/v1.23.0/bin/linux/amd64/kubectl \
    && chmod +x kubectl \
    && mv kubectl /usr/local/bin/

# Install AI/ML dependencies (TensorFlow, PyTorch, etc.)
RUN pip3 install torch torchvision numpy pandas scipy scikit-learn tensorflow keras

# Set up environment for cross-compilation and mobile development (iOS, Android)
RUN apt-get install -y \
    android-sdk \
    android-tools-adb \
    android-tools-fastboot \
    build-essential \
    libc6-dev-i386 \
    gcc-multilib \
    g++-multilib \
    lib32z1-dev \
    libncurses5-dev \
    libstdc++6:i386

# Set the working directory in the container
WORKDIR /workspace

# Set up Rust-specific environment (for Light Engine)
RUN rustup default stable \
    && rustup update \
    && rustup component add rust-src rustfmt clippy

# Expose necessary ports (e.g., for cloud services, simulators)
EXPOSE 8080

# Set up OpenGL and other graphics rendering tools
RUN apt-get install -y \
    libgl1-mesa-glx \
    libglu1-mesa \
    libgles2-mesa

# Install and set up persistent storage tools (e.g., SQLite, Redis)
RUN apt-get install -y sqlite3 redis-server

# Install additional tools for cloud simulation (e.g., message brokers, distributed systems)
RUN apt-get install -y \
    rabbitmq-server \
    nfs-common \
    samba \
    curl \
    jq

# Configure system for cloud-based container orchestration (Docker Swarm, Kubernetes)
RUN apt-get install -y \
    kubectl \
    helm

# Set up Docker Buildx
RUN mkdir -p ~/.docker/cli-plugins \
    && curl -SL https://github.com/docker/buildx/releases/download/v0.10.0/buildx-v0.10.0.linux-amd64 -o ~/.docker/cli-plugins/docker-buildx \
    && chmod +x ~/.docker/cli-plugins/docker-buildx

# Build and export multi-arch Docker image
RUN docker buildx create --use \
    && docker buildx build --platform linux/amd64,linux/arm64 --tag leonscott543/lightengine:latest --load .

# Default command (override this as necessary)
CMD ["bash"]
