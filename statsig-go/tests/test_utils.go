package tests

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func LoadTestData(path string) []byte {
	data, _ := os.ReadFile(path)
	return data
}

type testServerOptions struct {
	statusCode int
	dcsPath    string
	onLogEvent func(events []map[string]interface{})
}

func setupServer(opts testServerOptions) *httptest.Server {

	if opts.statusCode == 0 {
		opts.statusCode = http.StatusOK
	}
	if opts.dcsPath == "" {
		opts.dcsPath = "../../statsig-rust/tests/data/eval_proj_dcs.json"
	}
	dcsData := LoadTestData(opts.dcsPath)

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/v2/download_config_specs/secret-key.json":
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusOK)
			w.Write(dcsData)
		case "/v1/log_event":
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusOK)
			type requestInput struct {
				Events []map[string]interface{} `json:"events"`
			}
			input := &requestInput{}

			if r.Header.Get("Content-Encoding") == "gzip" {
				gz, _ := gzip.NewReader(r.Body)
				body, _ := io.ReadAll(gz)
				_ = json.Unmarshal(body, &input)
				gz.Close()
			} else {
				buf := new(bytes.Buffer)
				_, _ = buf.ReadFrom(r.Body)

				_ = json.Unmarshal(buf.Bytes(), &input)
			}

			if opts.onLogEvent != nil {
				opts.onLogEvent(input.Events)
			}
			w.Write([]byte(`{"success": true}`))
		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))

	return server
}

func checkEventNameExists(events []statsig.Event, eventName string) bool {
	for _, e := range events {
		if e.EventName == eventName {
			return true
		}
	}
	return false
}

func convertToExposureEvent(eventData map[string]interface{}) statsig.Event {
	eventJSON, err := json.Marshal(eventData)
	if err != nil {
		fmt.Println("Error marshalling:", err)
		return statsig.Event{}
	}
	var event statsig.Event
	if err := json.Unmarshal(eventJSON, &event); err != nil {
		fmt.Println("Error unmarshalling:", err)
		return statsig.Event{}
	}
	return event
}
