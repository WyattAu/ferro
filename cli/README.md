# Ferro CLI

## Installation

```bash
cargo install ferro-cli
```

## Usage

```bash
# List calendars
ferro calendars list

# Create calendar
ferro calendars create --name "My Calendar"

# List events
ferro events list --calendar default

# Create event
ferro events create --calendar default --summary "Meeting" --start "2024-01-01T10:00:00Z" --end "2024-01-01T11:00:00Z"

# List contacts
ferro contacts list

# Create contact
ferro contacts create --name "John Doe" --email "john@example.com"

# Sync status
ferro sync status

# Force sync
ferro sync force
```

## Configuration

```bash
# Set host
ferro config set host localhost

# Set port
ferro config set port 8080

# Set credentials
ferro config set username admin
ferro config set password password
```

## Output Formats

```bash
# JSON output
ferro calendars list --format json

# Table output (default)
ferro calendars list --format table

# CSV output
ferro calendars list --format csv
```
