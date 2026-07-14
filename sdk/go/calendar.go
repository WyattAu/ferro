package ferro

import "encoding/json"

// Calendar represents a calendar
type Calendar struct {
    ID          string `json:"id"`
    Name        string `json:"name"`
    Description string `json:"description,omitempty"`
}

// Event represents a calendar event
type Event struct {
    ID          string `json:"id"`
    CalendarID  string `json:"calendar_id"`
    Summary     string `json:"summary"`
    Start       string `json:"start"`
    End         string `json:"end"`
    Description string `json:"description,omitempty"`
}

// EventRequest holds event creation request
type EventRequest struct {
    Summary     string `json:"summary"`
    Start       string `json:"start"`
    End         string `json:"end"`
    Description string `json:"description,omitempty"`
}

// CalendarService handles calendar operations
type CalendarService struct {
    client *Client
}

// List returns all calendars
func (s *CalendarService) List() ([]Calendar, error) {
    data, err := s.client.doRequest("GET", "/dav/calendars", nil)
    if err != nil {
        return nil, err
    }

    var resp struct {
        Calendars []Calendar `json:"calendars"`
    }
    if err := json.Unmarshal(data, &resp); err != nil {
        return nil, err
    }

    return resp.Calendars, nil
}

// Get returns a calendar by ID
func (s *CalendarService) Get(id string) (*Calendar, error) {
    data, err := s.client.doRequest("GET", "/dav/calendars/"+id, nil)
    if err != nil {
        return nil, err
    }

    var calendar Calendar
    if err := json.Unmarshal(data, &calendar); err != nil {
        return nil, err
    }

    return &calendar, nil
}

// Create creates a new calendar
func (s *CalendarService) Create(name string) (*Calendar, error) {
    data, err := s.client.doRequest("POST", "/dav/calendars", map[string]string{"name": name})
    if err != nil {
        return nil, err
    }

    var calendar Calendar
    if err := json.Unmarshal(data, &calendar); err != nil {
        return nil, err
    }

    return &calendar, nil
}

// Delete deletes a calendar
func (s *CalendarService) Delete(id string) error {
    _, err := s.client.doRequest("DELETE", "/dav/calendars/"+id, nil)
    return err
}

// CreateEvent creates a new event
func (s *CalendarService) CreateEvent(calendarID string, req EventRequest) (*Event, error) {
    data, err := s.client.doRequest("POST", "/dav/calendars/"+calendarID+"/events", req)
    if err != nil {
        return nil, err
    }

    var event Event
    if err := json.Unmarshal(data, &event); err != nil {
        return nil, err
    }

    return &event, nil
}

// ListEvents returns all events in a calendar
func (s *CalendarService) ListEvents(calendarID string) ([]Event, error) {
    data, err := s.client.doRequest("GET", "/dav/calendars/"+calendarID+"/events", nil)
    if err != nil {
        return nil, err
    }

    var resp struct {
        Events []Event `json:"events"`
    }
    if err := json.Unmarshal(data, &resp); err != nil {
        return nil, err
    }

    return resp.Events, nil
}
