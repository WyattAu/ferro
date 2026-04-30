# CalDAV Clients

Connect your calendar and contacts apps to Ferro's CalDAV and CardDAV servers.

## Common Settings

For most clients, you need:

| Setting | Value |
|---------|-------|
| Server URL | `http://localhost:8080` (or your domain) |
| CalDAV path | `/dav/cal/` |
| CardDAV path | `/dav/card/` |
| Username | Your Ferro username |
| Password | Your Ferro password |

## Thunderbird (Cross-platform)

### Calendar setup

1. Open Thunderbird
2. Click the calendar icon in the toolbar
3. Right-click in the calendar list > New Calendar
4. Select "On the Network"
5. Choose "CalDAV"
6. Enter the server URL: `http://your-server:8080/dav/cal/`
7. Enter your credentials
8. Click "Find Calendars" to auto-discover
9. Select the calendar and click "Subscribe"

### Contacts setup

1. Open Thunderbird
2. Click the address book icon
3. File > New > Remote Address Book
4. Select "CardDAV"
5. Enter the server URL: `http://your-server:8080/dav/card/`
6. Enter your credentials
7. Click "OK"

## macOS Calendar

### Calendar setup

1. Open Calendar (or System Settings > Internet Accounts)
2. Click "Add Account" > "Other" > "CalDAV"
3. Select "Manual"
4. Enter:
   - Server: `your-server:8080`
   - Path: `/dav/cal/`
   - Username and password
5. Sign In

### Contacts setup

1. Open Contacts
2. Contacts > Add Account > Other Contacts Account
3. Select "CardDAV"
4. Enter:
   - Server: `your-server:8080`
   - Path: `/dav/card/`
   - Username and password
5. Sign In

## DAVx5 (Android)

### Setup

1. Install DAVx5 from F-Droid or Google Play
2. Open DAVx5
3. Tap "+" to add an account
4. Enter:
   - Base URL: `http://your-server:8080`
   - Username and password
5. DAVx5 will auto-discover CalDAV and CardDAV services
6. Select which calendars and address books to sync

### Troubleshooting

- Ensure the server URL is accessible from your device
- Check that `--admin-user` and `--admin-password` are set on the server
- DAVx5 requires HTTPS in production -- use a reverse proxy with TLS

## Evolution (Linux/GNOME)

### Calendar setup

1. Open Evolution
2. File > New > Calendar
3. Select "CalDAV"
4. Enter:
   - URL: `http://your-server:8080/dav/cal/`
   - Username and password
5. Click "Retrieve List" to find calendars

### Contacts setup

1. Open Evolution
2. File > New > Address Book
3. Select "CardDAV"
4. Enter:
   - URL: `http://your-server:8080/dav/card/`
   - Username and password
5. Click "Retrieve List" to find address books

## Troubleshooting

### Connection refused

- Ensure Ferro is running: `curl http://localhost:8080/healthz`
- Check the port matches your configuration

### Authentication failed

- Verify `--admin-user` and `--admin-password` are set
- Try the credentials with curl: `curl -u admin:password http://localhost:8080/api/config`

### Sync not working

- Check the CalDAV/CardDAV paths are correct
- Use `curl -X OPTIONS http://localhost:8080/dav/cal` to verify CalDAV is available
- Check server logs for errors

### HTTPS required

Many clients require HTTPS for CalDAV/CardDAV. Use a reverse proxy (Caddy, Nginx) with TLS certificates:

```bash
# Caddy (automatic HTTPS)
caddy reverse-proxy --from ferro.example.com --to localhost:8080
```
