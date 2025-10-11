#!/bin/bash
set -e

OPENSSL_VERSION="3.0.13"
ANDROID_API=21

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

if [ -z "$ANDROID_NDK_HOME" ]; then
    if [ -n "$ANDROID_NDK_ROOT" ]; then
        export ANDROID_NDK_HOME=$ANDROID_NDK_ROOT
    else
        log_error "ANDROID_NDK_HOME or ANDROID_NDK_ROOT not set"
        exit 1
    fi
fi

if [ ! -d "$ANDROID_NDK_HOME" ]; then
    log_error "NDK directory not found: $ANDROID_NDK_HOME"
    exit 1
fi

ARCH_NAME=""
OUTPUT_NAME=""

case "$1" in
    aarch64-linux-android)
        ARCH_NAME="android-arm64"
        OUTPUT_NAME="arm64-v8a"
        ;;
    armv7-linux-androideabi)
        ARCH_NAME="android-arm"
        OUTPUT_NAME="armeabi-v7a"
        ;;
    x86_64-linux-android)
        ARCH_NAME="android-x86_64"
        OUTPUT_NAME="x86_64"
        ;;
    i686-linux-android)
        ARCH_NAME="android-x86"
        OUTPUT_NAME="x86"
        ;;
    *)
        log_error "Usage: $0 <target>"
        log_error "  target: aarch64-linux-android, armv7-linux-androideabi, x86_64-linux-android, i686-linux-android"
        exit 1
        ;;
esac

WORK_DIR="/tmp/openssl_build_${OUTPUT_NAME}"
OUTPUT_DIR="/tmp/openssl_output_${OUTPUT_NAME}"

log_info "Building OpenSSL for ${OUTPUT_NAME}"
log_info "Target: $1"
log_info "NDK: $ANDROID_NDK_HOME"

mkdir -p "$WORK_DIR"
mkdir -p "$OUTPUT_DIR"

cd "$WORK_DIR"

if [ ! -f "openssl-${OPENSSL_VERSION}.tar.gz" ]; then
    log_info "Downloading OpenSSL ${OPENSSL_VERSION}..."
    wget -q --show-progress "https://www.openssl.org/source/openssl-${OPENSSL_VERSION}.tar.gz"
else
    log_info "OpenSSL tarball already exists"
fi

if [ ! -d "openssl-${OPENSSL_VERSION}" ]; then
    log_info "Extracting OpenSSL..."
    tar xzf "openssl-${OPENSSL_VERSION}.tar.gz"
fi

cd "openssl-${OPENSSL_VERSION}"

make distclean 2>/dev/null || true

export TOOLCHAIN=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64
export PATH=$TOOLCHAIN/bin:$PATH

case "$OUTPUT_NAME" in
    arm64-v8a)
        export CC=aarch64-linux-android${ANDROID_API}-clang
        export CXX=aarch64-linux-android${ANDROID_API}-clang++
        ;;
    armeabi-v7a)
        export CC=armv7a-linux-androideabi${ANDROID_API}-clang
        export CXX=armv7a-linux-androideabi${ANDROID_API}-clang++
        ;;
    x86_64)
        export CC=x86_64-linux-android${ANDROID_API}-clang
        export CXX=x86_64-linux-android${ANDROID_API}-clang++
        ;;
    x86)
        export CC=i686-linux-android${ANDROID_API}-clang
        export CXX=i686-linux-android${ANDROID_API}-clang++
        ;;
esac

export AR=llvm-ar
export RANLIB=llvm-ranlib

log_info "Configuring OpenSSL..."
./Configure ${ARCH_NAME} \
    -D__ANDROID_API__=${ANDROID_API} \
    --prefix="${OUTPUT_DIR}" \
    shared \
    no-tests

log_info "Compiling OpenSSL (this may take several minutes)..."
make -j$(nproc)

log_info "Installing to ${OUTPUT_DIR}..."
make install_sw install_ssldirs

if [ -f "${OUTPUT_DIR}/lib/libssl.so" ]; then
    log_info "Build completed successfully"
    log_info "Output directory: ${OUTPUT_DIR}"
    log_info "Libraries:"
    ls -lh "${OUTPUT_DIR}/lib/"*.so*
    echo ""
    echo "export OPENSSL_DIR=${OUTPUT_DIR}"
    echo "export OPENSSL_LIB_DIR=${OUTPUT_DIR}/lib"
else
    log_error "Build failed: libssl.so not found"
    exit 1
fi

exit 0
