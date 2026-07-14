package ferro

import (
    "bytes"
    "encoding/json"
    "fmt"
    "io"
    "net/http"
)

// ClientConfig holds client configuration
type ClientConfig struct {
    Host     string
    Port     int
    Username string
    Password string
}

// Client is the Ferro API client
type Client struct {
    config     ClientConfig
    httpClient *http.Client
    Calendars  *CalendarService
    Contacts   *ContactService
}

// NewClient creates a new Ferro client
func NewClient(config ClientConfig) *Client {
    client := &Client{
        config:     config,
        httpClient: &http.Client{},
    }
    client.Calendars = &CalendarService{client: client}
    client.Contacts = &ContactService{client: client}
    return client
}

// doRequest performs an HTTP request
func (c *Client) doRequest(method, path string, body interface{}) ([]byte, error) {
    url := fmt.Sprintf("http://%s:%d%s", c.config.Host, c.config.Port, path)

    var reqBody []byte
    if body != nil {
        var err error
        reqBody, err = json.Marshal(body)
        if err != nil {
            return nil, err
        }
    }

    req, err := http.NewRequest(method, url, bytes.NewBuffer(reqBody))
    if err != nil {
        return nil, err
    }

    req.SetBasicAuth(c.config.Username, c.config.Password)
    req.Header.Set("Content-Type", "application/json")

    resp, err := c.httpClient.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    return io.ReadAll(resp.Body)
}
