#!/bin/sh
set -e

# Default UID/GID (OpenShift nonroot)
APP_UID=${APP_UID:-65532}
APP_GID=${APP_GID:-65532}

# Only modify user if UID/GID differ from default
if [ "$APP_UID" != "65532" ] || [ "$APP_GID" != "65532" ]; then
    # Create group if it doesn't exist
    if ! getent group $APP_GID > /dev/null 2>&1; then
        addgroup -g $APP_GID appgroup
    fi
    
    # Create user if it doesn't exist
    if ! getent passwd $APP_UID > /dev/null 2>&1; then
        adduser -u $APP_UID -G appgroup -D appuser
    fi
    
    # Fix ownership
    chown -R $APP_UID:$APP_GID /data 2>/dev/null || true
    chown -R $APP_UID:$APP_GID /app 2>/dev/null || true
    
    # Drop privileges and exec
    exec su-exec $APP_UID:$APP_GID "$@"
fi

# Already running as correct user, just exec
exec "$@"
