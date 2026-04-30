# Firecracker Deployment

Deploy Ferro inside a Firecracker MicroVM for VM-level isolation with minimal attack surface and ~125ms boot time.

## Prerequisites

- Firecracker v1.6+
- Root privileges
- `tuntap` kernel module

## Quick Start

```bash
cd deploy/firecracker
chmod +x start-vm.sh
sudo ./start-vm.sh
```

Ferro will be available at `http://<VM-IP>:8080`.

## Configuration

Environment variables control the MicroVM:

| Variable | Default | Description |
|----------|---------|-------------|
| `FIRECRACKER_KERNEL` | `/opt/ferro/vmlinux` | Path to kernel image |
| `FIRECRACKER_ROOTFS` | `/opt/ferro/rootfs.ext4` | Path to root filesystem |
| `FIRECRACKER_SOCKET` | `/tmp/firecracker.sock` | API socket path |
| `FIRECRACKER_TAP` | `tap0` | TAP device name |
| `FIRECRACKER_VCPUS` | `2` | Number of vCPUs |
| `FIRECRACKER_MEM` | `512` | Memory in MiB |
| `FIRECRACKER_MAC` | `AA:FC:00:00:00:01` | Guest MAC address |
| `FIRECRACKER_ROOTFS_SIZE` | `512` | Rootfs size in MiB |

## Resource Requirements

| Resource | Requirement |
|----------|-------------|
| vCPUs | 2 |
| RAM | 512 MB |
| Disk (rootfs) | 512 MB |

## Building the Root Filesystem

The root filesystem is built using a Dockerfile:

```bash
cd deploy/firecracker/ferro-rootfs
docker build -t ferro-rootfs .
```

## Security Benefits

- VM-level isolation (separate kernel)
- Minimal attack surface (only networking and block device)
- No shared kernel with host
- Fast boot (~125ms) for quick scaling
- No persistent state on host beyond rootfs

## Tips

- Use a TAP device for network access from the host
- The VM has no persistent storage beyond the rootfs -- mount external storage or use S3
- Configure the firewall to restrict access to the VM IP
- Monitor VM resource usage via the Firecracker API socket
