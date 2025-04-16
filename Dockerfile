# Start with an official Ubuntu image as a base (you can change this as per your needs)
FROM ubuntu:20.04

LABEL Name=lightengine Version=0.0.1

# Install necessary system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    cmake \
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
    wget \
    curl \
    unzip \
    gnupg \
    libclang-dev

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Vulkan SDK (Replace this with the actual URL you need)
RUN wget https://sdk.lunarg.com/sdk/download/latest/linux/vulkan-sdk.tar.xz -O /tmp/vulkan-sdk.tar.xz \
    && mkdir /tmp/vulkan-sdk \
    && tar -xvf /tmp/vulkan-sdk.tar.xz -C /tmp/vulkan-sdk \
    && cp -r /tmp/vulkan-sdk/vulkan-sdk/* /usr/local/

# Set Vulkan environment variables
ENV VULKAN_SDK=/usr/local/vulkan-sdk/x.x.x.x/x86_64  # Replace with the correct SDK version
ENV PATH=$VULKAN_SDK/bin:$PATH
ENV LD_LIBRARY_PATH=$VULKAN_SDK/lib:$LD_LIBRARY_PATH
ENV VK_ICD_FILENAMES=$VULKAN_SDK/etc/vulkan/icd.d/nvidia_icd.json
ENV VK_LAYER_PATH=$VULKAN_SDK/etc/vulkan/explicit_layer.d

# Install Rust dependencies (optional for your project)
RUN rustup default stable

# Set up workspace directory
WORKDIR /workspace

# Expose necessary ports (if applicable, adjust as needed)
EXPOSE 8080

# Default command (use your actual app here or leave it open for development)
CMD ["bash"]
