#!/bin/bash

set -eu

echo "Building UEFI firmware"

# Compile firmware and save it for later
EDK2_VERSION=edk2-stable202211
git clone --depth 1 -b "$EDK2_VERSION" git@github.com:tianocore/edk2.git $TOOLCHAIN_BUILD_DIR/edk2
pushd $TOOLCHAIN_BUILD_DIR/edk2
git submodule update --init --depth 1
OvmfPkg/build.sh -a x64 -t GCC5 -b RELEASE
cp -v Build/OvmfX64/RELEASE_GCC5/FV/OVMF.fd ../
popd
exit 0
