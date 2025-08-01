package tests

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"sync"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
)

type MockScrapi struct {
	mocks          map[string]MockResponse
	lock           sync.Mutex
	server         httptest.Server
	loggedRequests []MockRequest
	events         []statsig.Event
}

type MockResponse struct {
	StatusCode int
	Body       []byte
	Callback   func(w http.ResponseWriter, r *http.Request) bool
}

type MockRequest struct {
	Endpoint string
	Method   string
	Body     []byte
}

func NewMockScrapiServer() *MockScrapi {
	m := &MockScrapi{
		mocks:          make(map[string]MockResponse),
		loggedRequests: []MockRequest{},
		events:         []statsig.Event{},
	}
	m.server = *httptest.NewServer(http.HandlerFunc(m.Handler))
	return m
}

func (scrapi *MockScrapi) Mock(endpoint string, statusCode int, res_data []byte) {
	scrapi.lock.Lock()
	defer scrapi.lock.Unlock()

	res := MockResponse{StatusCode: statusCode, Body: res_data, Callback: nil}
	scrapi.mocks[endpoint] = res
}

func (scrapi *MockScrapi) Handler(w http.ResponseWriter, r *http.Request) {
	scrapi.lock.Lock()
	defer scrapi.lock.Unlock()

	body, _ := io.ReadAll(r.Body)

	req := MockRequest{Endpoint: r.URL.Path, Method: r.Method, Body: body}
	scrapi.loggedRequests = append(scrapi.loggedRequests, req)

	saved_res, ok := scrapi.mocks[r.URL.Path]

	if !ok {
		w.WriteHeader(http.StatusNotFound)
		return
	}

	if r.URL.Path == "/v1/log_event" {
		scrapi.addEvent(r.Header, body)
	}

	_ = r.Body.Close()

	w.WriteHeader(saved_res.StatusCode)
	_, _ = w.Write(saved_res.Body)

}

func (scrapi *MockScrapi) Reset() {
	scrapi.lock.Lock()
	defer scrapi.lock.Unlock()

	scrapi.mocks = make(map[string]MockResponse)
	scrapi.loggedRequests = []MockRequest{}
	scrapi.events = []statsig.Event{}
}

func (scrapi *MockScrapi) FetchLoggedRequests() []MockRequest {
	scrapi.lock.Lock()
	defer scrapi.lock.Unlock()

	return scrapi.loggedRequests
}

func (scrapi *MockScrapi) GetUrlForEndpoint(endpoint string) string {
	scrapi.lock.Lock()
	defer scrapi.lock.Unlock()

	return scrapi.server.URL + endpoint
}
func (scrapi *MockScrapi) fetchLoggedEvents() []statsig.Event {
	scrapi.lock.Lock()
	defer scrapi.lock.Unlock()

	return scrapi.events
}

func (scrapi *MockScrapi) addEvent(header http.Header, body []byte) {
	type requestInput struct {
		Events []statsig.Event `json:"events"`
	}
	var input requestInput

	if header.Get("Content-Encoding") == "gzip" {
		gz, _ := gzip.NewReader(bytes.NewReader(body))
		body, _ := io.ReadAll(gz)
		_ = json.Unmarshal(body, &input)
		_ = gz.Close()
	} else {
		_ = json.Unmarshal(body, &input)
	}

	scrapi.events = append(scrapi.events, input.Events...)
}
