package ferro

import "encoding/json"

// Contact represents a contact
type Contact struct {
    ID    string `json:"id"`
    Name  string `json:"name"`
    Email string `json:"email,omitempty"`
    Phone string `json:"phone,omitempty"`
}

// ContactService handles contact operations
type ContactService struct {
    client *Client
}

// List returns all contacts
func (s *ContactService) List() ([]Contact, error) {
    data, err := s.client.doRequest("GET", "/dav/contacts", nil)
    if err != nil {
        return nil, err
    }

    var resp struct {
        Contacts []Contact `json:"contacts"`
    }
    if err := json.Unmarshal(data, &resp); err != nil {
        return nil, err
    }

    return resp.Contacts, nil
}

// Get returns a contact by ID
func (s *ContactService) Get(id string) (*Contact, error) {
    data, err := s.client.doRequest("GET", "/dav/contacts/"+id, nil)
    if err != nil {
        return nil, err
    }

    var contact Contact
    if err := json.Unmarshal(data, &contact); err != nil {
        return nil, err
    }

    return &contact, nil
}

// Create creates a new contact
func (s *ContactService) Create(name string) (*Contact, error) {
    data, err := s.client.doRequest("POST", "/dav/contacts", map[string]string{"name": name})
    if err != nil {
        return nil, err
    }

    var contact Contact
    if err := json.Unmarshal(data, &contact); err != nil {
        return nil, err
    }

    return &contact, nil
}

// Delete deletes a contact
func (s *ContactService) Delete(id string) error {
    _, err := s.client.doRequest("DELETE", "/dav/contacts/"+id, nil)
    return err
}
