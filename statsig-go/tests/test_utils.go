package tests

import (
	"net/http"
	"net/http/httptest"
	"os"
)

func LoadTestData(path string) []byte {
	data, _ := os.ReadFile(path)
	return data
}

func setupServer() *httptest.Server {

	dcsData := LoadTestData("../../statsig-rust/tests/data/eval_proj_dcs.json")

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/v2/download_config_specs/secret-key.json":
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusOK)
			w.Write(dcsData)
		case "/v1/log_event":
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusOK)
			w.Write([]byte(`{"success": true}`))
		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))

	return server
}
