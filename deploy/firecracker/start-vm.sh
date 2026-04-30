#!/bin/bash
set -euo pipefail

KERNEL="${FIRECRACKER_KERNEL:-/opt/ferro/vmlinux}"
ROOTFS="${FIRECRACKER_ROOTFS:-/opt/ferro/rootfs.ext4}"
BOOT_ARGS="console=ttyS0 reboot=k panic=1 pci=off i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd apm=off"
FC_API="${FIRECRACKER_SOCKET:-/tmp/firecracker.sock}"
TAP_DEV="${FIRECRACKER_TAP:-tap0}"
VCPUS="${FIRECRACKER_VCPUS:-2}"
MEM_SIZE="${FIRECRACKER_MEM:-512}"
GUEST_MAC="${FIRECRACKER_MAC:-AA:FC:00:00:00:01}"
ROOTFS_SIZE="${FIRECRACKER_ROOTFS_SIZE:-512}"

if [ ! -f "$ROOTFS" ]; then
    echo "Building rootfs..."
    dd if=/dev/zero of="$ROOTFS" bs=1M count="$ROOTFS_SIZE"
    mkfs.ext4 "$ROOTFS"
    mkdir -p /tmp/rootfs
    mount -o loop "$ROOTFS" /tmp/rootfs
    cp /usr/local/bin/ferro-server /tmp/rootfs/usr/local/bin/
    cp /usr/local/bin/ferro-cli /tmp/rootfs/usr/local/bin/
    mkdir -p /tmp/rootfs/data
    umount /tmp/rootfs
    rmdir /tmp/rootfs
    echo "Rootfs created at $ROOTFS"
fi

curl --unix-socket "$FC_API" -X PUT "http://localhost/boot-source" \
    -H "Content-Type: application/json" \
    -d "{\"kernel_image_path\": \"$KERNEL\", \"boot_args\": \"$BOOT_ARGS\"}"

curl --unix-socket "$FC_API" -X PUT "http://localhost/drives/rootfs" \
    -H "Content-Type: application/json" \
    -d "{\"drive_id\": \"rootfs\", \"path_on_host\": \"$ROOTFS\", \"is_root_device\": true, \"is_read_only\": false}"

curl --unix-socket "$FC_API" -X PUT "http://localhost/machine-config" \
    -H "Content-Type: application/json" \
    -d "{\"vcpu_count\": $VCPUS, \"mem_size_mib\": $MEM_SIZE}"

curl --unix-socket "$FC_API" -X PUT "http://localhost/network-interfaces/eth0" \
    -H "Content-Type: application/json" \
    -d "{\"iface_id\": \"eth0\", \"host_dev_name\": \"$TAP_DEV\", \"guest_mac\": \"$GUEST_MAC\"}"

curl --unix-socket "$FC_API" -X PUT "http://localhost/actions" \
    -H "Content-Type: application/json" \
    -d "{\"action_type\": \"InstanceStart\"}"
