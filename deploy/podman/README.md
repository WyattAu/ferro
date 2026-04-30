# Podman Deployment

## Rootless Setup

```bash
podman-compose -f podman-compose.yml up -d
```

## With Podman Machine (macOS/Windows)

```bash
podman machine init
podman machine start
eval $(podman machine env)
podman-compose -f podman-compose.yml up -d
```

## Systemd Integration

```bash
podman generate systemd --new --files --name ferro
cp container-ferro.service ~/.config/systemd/user/
systemctl --user enable --now container-ferro.service
```
