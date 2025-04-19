# Light Engine: Developer & Deployment Setup Guide

## Developer Setup (Native or Dev Container)

### Prerequisites

#### Dev Container
- **Rust** (with Cargo) – pre-installed and available on the `PATH`.
- **Python 3 & pip3** – pre-installed and available on the `PATH`.
- **Git** – pre-installed and available on the `PATH`.

#### Native Setup (System Dependencies)

##### Linux (Ubuntu/Debian)
Install the required dependencies:
```sh
sudo apt-get update && sudo apt-get install -y \
    build-essential git curl cmake wget unzip python3-pip \

```

##### Windows
1. Install Vulkan SDK.
2. Install Rust.
3. Install Python 3.
4. Install dependencies via vcpkg or the appropriate installers.

##### macOS
1. Metal is built-in (for wgpu/ash).
2. Install dependencies via Homebrew:
   ```sh
   brew install cmake python3 rust git vulkan-sdk
   ```

---

### Clone and Build
1. Clone the repository:
   ```sh
   git clone https://github.com/your-repo/light-engine.git
   cd light-engine
   ```
2. Build the project using the appropriate build system.

---

### Run
- **Native**: Follow the build instructions and execute the binary.
- **Docker**: Use the provided Docker image.

---

### Running with Docker (Images)

#### Prerequisites
- Docker installed on your system.

#### Build the Image (if not using prebuilt)
```sh
docker build -t light-engine .
```

#### Run the Image
```sh
docker run -p 8080:8080 light-engine
```
The application will be available on port `8080` (or as configured). The image supports both `amd64` and `arm64` platforms.

---

### Notes

#### Dev Container
If using VS Code Dev Containers, all tools and dependencies are pre-installed. Just open the folder in VS Code and start developing!

#### Native GPU Support
For Vulkan or Metal support, ensure your host system has the appropriate drivers and hardware.

#### AI/ML Libraries
Python ML libraries (e.g., `torch`, `tensorflow`) are pre-installed in the Docker image and dev container.