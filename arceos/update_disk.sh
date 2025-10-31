#!/bin/sh

if [ $# -ne 1 ]; then
    printf "Usage: ./update.sh [userapp path]\n"
    exit
fi

FILE=$1

if [ ! -f $FILE ]; then
    printf "File '$FILE' doesn't exist!\n"
    exit
fi

if [ ! -f ./disk.img ]; then
    printf "disk.img doesn't exist! Please 'make disk_img'\n"
    exit
fi

printf "Write file '$FILE' into disk.img\n"

# Try to use mount first, fallback to mtools if mount fails
mkdir -p ./mnt
if sudo mount ./disk.img ./mnt 2>/dev/null && mountpoint -q ./mnt 2>/dev/null; then
    # Mount succeeded, use traditional method
    sudo mkdir -p ./mnt/sbin
    sudo cp "$FILE" ./mnt/sbin
    sync
    sudo umount ./mnt
    sync
    rm -rf mnt
    printf "Successfully copied using mount\n"
else
    # Mount failed (no loop device), use mtools instead
    printf "Mount failed, using mtools instead...\n"
    sudo umount ./mnt 2>/dev/null || true
    rm -rf mnt
    # Create /sbin directory if it doesn't exist
    mmd -i disk.img ::/sbin 2>/dev/null || true
    # Copy file using mcopy
    mcopy -i disk.img -o "$FILE" ::/sbin/
    printf "Successfully copied using mtools\n"
fi
