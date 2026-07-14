# Ferro Go SDK

## Installation

```bash
go get github.com/WyattAu/ferro/sdk/go
```

## Quick Start

```go
package main

import (
    "fmt"
    ferro "github.com/WyattAu/ferro/sdk/go"
)

func main() {
    client := ferro.NewClient(ferro.ClientConfig{
        Host:     "localhost",
        Port:     8080,
        Username: "admin",
        Password: "password",
    })

    // List calendars
    calendars, err := client.Calendars.List()
    if err != nil {
        panic(err)
    }

    for _, calendar := range calendars {
        fmt.Println(calendar.Name)
    }

    // Create event
    event, err := client.Calendars.CreateEvent("default", ferro.EventRequest{
        Summary: "Team Meeting",
        Start:   "2024-01-01T10:00:00Z",
        End:     "2024-01-01T11:00:00Z",
    })
    if err != nil {
        panic(err)
    }

    fmt.Println(event.ID)
}
```

## API Reference

### Client

```go
type Client struct {
    Calendars *CalendarService
    Contacts  *ContactService
}

func NewClient(config ClientConfig) *Client
```

### CalendarService

```go
type CalendarService struct {
    List() ([]Calendar, error)
    Get(id string) (*Calendar, error)
    Create(name string) (*Calendar, error)
    Delete(id string) error
    CreateEvent(calendarID string, req EventRequest) (*Event, error)
    ListEvents(calendarID string) ([]Event, error)
}
```

### ContactService

```go
type ContactService struct {
    List() ([]Contact, error)
    Get(id string) (*Contact, error)
    Create(name string) (*Contact, error)
    Delete(id string) error
}
```
