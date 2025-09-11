#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VERSION=${1:-"0.1.0"}
ARCHITECTURES=("amd64" "arm64")

echo -e "${BLUE}🔨 Building cert-agent packages version $VERSION${NC}"

# Function to print colored output
print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Check if we're on a supported platform
check_platform() {
    case "$(uname -m)" in
        x86_64)
            HOST_ARCH="amd64"
            ;;
        arm64|aarch64)
            HOST_ARCH="arm64"
            ;;
        *)
            print_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
    print_status "Detected host architecture: $HOST_ARCH"
}

# Install cross-compilation dependencies
install_dependencies() {
    print_status "Installing build dependencies..."
    
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt-get update
        sudo apt-get install -y \
            libssl-dev \
            pkg-config \
            protobuf-compiler \
            perl \
            make \
            build-essential \
            debhelper \
            devscripts \
            dh-make \
            gcc-aarch64-linux-gnu \
            libc6-dev-arm64-cross
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS - install via Homebrew
        brew install dpkg
        print_warning "Cross-compilation from macOS is limited. Consider using GitHub Actions."
    fi
}

# Build for specific architecture
build_architecture() {
    local arch=$1
    local rust_target=$2
    local deb_arch=$3
    
    echo -e "${BLUE}🏗️  Building for $arch ($rust_target)${NC}"
    
    # Clean previous builds
    cargo clean
    
    # Configure cross-compilation if needed
    if [[ "$arch" == "arm64" && "$HOST_ARCH" == "amd64" ]]; then
        print_status "Configuring ARM64 cross-compilation..."
        rustup target add aarch64-unknown-linux-gnu
        
        # Create cargo config for cross-compilation
        mkdir -p .cargo
        cat > .cargo/config.toml << EOF
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
EOF
    fi
    
    # Build Rust application
    print_status "Building Rust application..."
    export PKG_CONFIG_ALLOW_CROSS=1
    cargo build --release --target $rust_target
    
    # Prepare debian package
    print_status "Preparing Debian package..."
    
    # Update version in changelog
    sed -i.bak "s/0\.1\.0-1/$VERSION-1/" debian/changelog
    sed -i.bak "s/Architecture: amd64 arm64/Architecture: $deb_arch/" debian/control
    
    # Create package structure
    mkdir -p debian/cert-agent/usr/bin
    cp target/$rust_target/release/cert-agent debian/cert-agent/usr/bin/
    chmod +x debian/cert-agent/usr/bin/cert-agent
    
    # Build Debian package
    print_status "Building Debian package..."
    dpkg-buildpackage -us -uc -b --host-arch $deb_arch
    
    # Move package to output directory
    mkdir -p packages/$arch
    mv ../cert-agent_*_$deb_arch.deb packages/$arch/
    
    print_status "Package built successfully: packages/$arch/"
    
    # Restore original files
    mv debian/changelog.bak debian/changelog
    mv debian/control.bak debian/control
}

# Main build process
main() {
    check_platform
    install_dependencies
    
    # Create output directory
    rm -rf packages
    mkdir -p packages
    
    # Build for each architecture
    for arch in "${ARCHITECTURES[@]}"; do
        case $arch in
            amd64)
                build_architecture "amd64" "x86_64-unknown-linux-gnu" "amd64"
                ;;
            arm64)
                build_architecture "arm64" "aarch64-unknown-linux-gnu" "arm64"
                ;;
        esac
    done
    
    # List created packages
    echo -e "${BLUE}📦 Created packages:${NC}"
    find packages -name "*.deb" -exec ls -lh {} \;
    
    # Generate checksums
    print_status "Generating checksums..."
    find packages -name "*.deb" -exec sha256sum {} \; > packages/SHA256SUMS
    
    echo -e "${GREEN}🎉 All packages built successfully!${NC}"
    echo -e "${BLUE}Packages are available in the 'packages' directory${NC}"
}

# Run main function
main "$@"
