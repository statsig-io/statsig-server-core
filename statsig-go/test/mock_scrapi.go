package test

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"time"
)

const VERBOSE = false

type RecordedRequest struct {
	Method   string
	Path     string
	RawQuery string
	Header   http.Header
	Body     []byte
}

type StubResponse struct {
	Status int
	Header http.Header
	Body   []byte
	Delay  time.Duration
}

type key struct {
	method string
	path   string
}

type MockScrapi struct {
	srv      *httptest.Server
	mu       sync.Mutex
	requests []RecordedRequest
	stubs    map[key][]StubResponse // FIFO per key
	fallback *StubResponse
	events   []map[string]any
}

// New creates a running mock server.
func NewMockScrapi() *MockScrapi {
	m := &MockScrapi{
		stubs: make(map[key][]StubResponse),
	}
	m.srv = httptest.NewServer(http.HandlerFunc(m.handle))
	return m
}

func (m *MockScrapi) URL() string { return m.srv.URL }
func (m *MockScrapi) Close()      { m.srv.Close() }

// Stub queues a response for the given method+path.
// Each incoming match pops the next queued response.
func (m *MockScrapi) Stub(method, path string, resp StubResponse) {
	m.mu.Lock()
	defer m.mu.Unlock()
	k := key{method: method, path: path}
	m.stubs[k] = append(m.stubs[k], resp)
}

// SetFallback sets a response used when no stub matches.
// Pass zero value to clear.
func (m *MockScrapi) SetFallback(resp *StubResponse) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.fallback = resp
}

// Requests returns a copy of all recorded requests in arrival order.
func (m *MockScrapi) Requests() []RecordedRequest {
	m.mu.Lock()
	defer m.mu.Unlock()
	out := make([]RecordedRequest, len(m.requests))
	copy(out, m.requests)
	return out
}

// ClearRequests removes all recorded requests.
func (m *MockScrapi) ClearRequests() {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.requests = m.requests[:0]
}

func (m *MockScrapi) Events() []map[string]any {
	m.mu.Lock()
	defer m.mu.Unlock()

	// filter out diagnostics events
	filteredEvents := make([]map[string]any, 0)
	for _, event := range m.events {
		if event["eventName"] != "statsig::diagnostics" {
			filteredEvents = append(filteredEvents, event)
		}
	}
	return filteredEvents
}

func (m *MockScrapi) EventsWithDiagnostics() []map[string]any {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.events
}

func (m *MockScrapi) ClearEvents() {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.events = m.events[:0]
}

// ClearStubs removes all queued stubs and fallback.
func (m *MockScrapi) ClearStubs() {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.stubs = make(map[key][]StubResponse)
	m.fallback = nil
}

func (m *MockScrapi) ClearAll() {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.requests = m.requests[:0]
	m.events = m.events[:0]
	m.stubs = make(map[key][]StubResponse)
	m.fallback = nil
}

// Helpers to build common stub responses.

func JSON(status int, v any) StubResponse {
	b, _ := json.Marshal(v)
	h := http.Header{}
	h.Set("Content-Type", "application/json")
	return StubResponse{Status: status, Header: h, Body: b}
}

func Text(status int, s string) StubResponse {
	h := http.Header{}
	h.Set("Content-Type", "text/plain; charset=utf-8")
	return StubResponse{Status: status, Header: h, Body: []byte(s)}
}

func Bytes(status int, body []byte, hdr http.Header) StubResponse {
	// Copies header for safety
	h := http.Header{}
	for k, vals := range hdr {
		for _, v := range vals {
			h.Add(k, v)
		}
	}
	return StubResponse{Status: status, Header: h, Body: append([]byte(nil), body...)}
}

// Internal handler.
func (m *MockScrapi) handle(w http.ResponseWriter, r *http.Request) {
	body := readRequestBody(r)

	if VERBOSE {
		fmt.Println("Received request:", r.URL.Path)
		fmt.Println("Headers:", r.Header)
		fmt.Println("Body:", string(body))
	}

	if strings.Contains(r.URL.Path, "log_event") {
		m.tryAddEvents(body)
	}

	// record
	rec := RecordedRequest{
		Method:   r.Method,
		Path:     r.URL.Path,
		RawQuery: r.URL.RawQuery,
		Header:   cloneHeader(r.Header),
		Body:     append([]byte(nil), body...),
	}

	var resp *StubResponse

	m.mu.Lock()
	m.requests = append(m.requests, rec)

	k := key{method: r.Method, path: r.URL.Path}
	queue := m.stubs[k]
	if len(queue) > 0 {
		resp = &queue[0]
		// pop
		if len(queue) == 1 {
			delete(m.stubs, k)
		} else {
			m.stubs[k] = queue[1:]
		}
	} else if m.fallback != nil {
		resp = m.fallback
	}
	m.mu.Unlock()

	// no match
	if resp == nil {
		http.NotFound(w, r)
		return
	}

	if resp.Delay > 0 {
		time.Sleep(resp.Delay)
	}

	for k, vals := range resp.Header {
		for _, v := range vals {
			w.Header().Add(k, v)
		}
	}
	status := resp.Status
	if status == 0 {
		status = http.StatusOK
	}
	w.WriteHeader(status)
	_, _ = io.Copy(w, bytes.NewReader(resp.Body))
}

func (m *MockScrapi) tryAddEvents(body []byte) {
	bodyMap := make(map[string]any)
	err := json.Unmarshal(body, &bodyMap)
	if err != nil {
		return
	}

	events := bodyMap["events"].([]any)
	if len(events) == 0 {
		return
	}

	events_map := make([]map[string]any, len(events))
	for i, event := range events {
		events_map[i] = event.(map[string]any)
	}

	m.events = append(m.events, events_map...)
}

func cloneHeader(h http.Header) http.Header {
	cp := http.Header{}
	for k, vals := range h {
		for _, v := range vals {
			cp.Add(k, v)
		}
	}
	return cp
}

func readRequestBody(r *http.Request) []byte {
	encoding := r.Header.Get("Content-Encoding")

	if encoding == "gzip" {
		gr, _ := gzip.NewReader(r.Body)
		defer gr.Close()
		defer r.Body.Close()

		data, err := io.ReadAll(gr)
		if err != nil {
			return nil
		}

		return data
	}

	body, _ := io.ReadAll(r.Body)
	defer r.Body.Close()
	return body
}
