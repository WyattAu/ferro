# Firecracker MicroVM Deployment

Firecracker provides VM-level isolation with ~125ms boot time.

## Prerequisites
- Firecracker v1.6+
- Root privileges
- `tuntap` kernel module

## Usage
```bash
chmod +x start-vm.sh
sudo ./start-vm.sh
```

Ferro will be available at http://<VM-IP>:8080

## Configuration

Environment variables:
| Variable | Default | Description |
|---|---|---|
| `FIRECRACKER_KERNEL` | `/opt/ferro/vmlinux` | Path to kernel image |
| `FIRECRACKER_ROOTFS` | `/opt/ferro/rootfs.ext4` | Path to root filesystem |
| `FIRECRACKER_SOCKET` | `/tmp/firecracker.sock` | API socket path |
| `FIRECRACKER_TAP` | `tap0` | TAP device name |
| `FIRECRACKER_VCPUS` | `2` | Number of vCPUs |
| `FIRECRACKER_MEM` | `512` | Memory in MiB |
| `FIRECRACKER_MAC` | `AA:FC:00:00:00:01` | Guest MAC address |
| `FIRECRACKER_ROOTFS_SIZE` | `512` | Rootfs size in MiB |

## Resource Requirements
- 2 vCPUs
- 512 MB RAM
- 512 MB disk (rootfs)
