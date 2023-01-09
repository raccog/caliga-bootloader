#!/bin/bash

set -e

OVMF_DST_PATH=build-external/OVMF.fd
echo "Building UEFI firmware"

# Compile firmware and save it for later
EDK2_VERSION=edk2-stable202211
git clone --depth 1 -b "$EDK2_VERSION" git@github.com:tianocore/edk2.git build-external/edk2
pushd build-external/edk2
git submodule update --init --depth 1
OvmfPkg/build.sh -a x64 -t GCC5 -b RELEASE
cp Build/OvmfX64/RELEASE_GCC5/FV/OVMF.fd ../
popd
exit 0
