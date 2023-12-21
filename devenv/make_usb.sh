#!/bin/sh -ex

if [ $# -lt 2 ]; then
	echo "Usage: $0 <drive> <.efi file> [another file]"
	exit 1
fi

DEVENV_DIR=$(dirname "$0")
DRIVE=$1
EFI_FILE=$2
ANOTHER_FILE=$3
MOUNT_POINT=./mnt

if [ ! -f $EFI_FILE ]; then
	echo "No such file: $EFI_FILE"
	exit 1
fi

mkdir -p $MOUNT_POINT

sudo mount -t drvfs $DRIVE $MOUNT_POINT
sudo rm -rf $MOUNT_POINT/*
sudo mkdir -p $MOUNT_POINT/EFI/BOOT

sudo cp $EFI_FILE $MOUNT_POINT/EFI/BOOT/BOOTX64.EFI
if [ "$ANOTHER_FILE" != "" ]; then
	sudo cp $ANOTHER_FILE $MOUNT_POINT/
fi
sleep 0.5
sudo umount $MOUNT_POINT