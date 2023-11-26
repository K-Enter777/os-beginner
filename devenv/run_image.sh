#!/bin/sh -ex

if [ $# -lt 1 ]; then
	echo "Usage: $0 <image name>"
	exit 1
fi

DEVENV_DIR=$(dirname "$0")
DISK_IMG=$1

if [ ! -f $DISK_IMG ]; then
	echo "No such file: $DISK_IMG"
	exit 1
fi

qemu-system-x86_64 \
	-drive if=pflash,format=raw,readonly=on,file=$DEVENV_DIR/OVMF_CODE.fd \
	-drive if=pflash,format=raw,file=$DEVENV_DIR/OVMF_VARS.fd \
	-monitor stdio \
	-hda $DISK_IMG