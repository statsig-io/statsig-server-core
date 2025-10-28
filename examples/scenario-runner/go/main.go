package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"sort"
	"time"
)

var PROFILE_ARR []map[string]interface{}

type GateConfig struct {
	Names []string `json:"names"`
	QPS   int      `json:"qps"`
}

type LogEventConfig struct {
	Events map[string]Event `json:"events"`
	QPS    int              `json:"qps"`
}

type Event struct {
	EventName string `json:"eventName"`
}

type GCIRConfig struct {
	QPS int `json:"qps"`
}

type SdkState struct {
	Users    map[string]UserData `json:"users"`
	Gate     GateConfig          `json:"gate"`
	LogEvent LogEventConfig      `json:"logEvent"`
	GCIR     GCIRConfig          `json:"gcir"`
}

type UserData struct {
	UserID string `json:"userID"`
}

type State struct {
	SDK SdkState `json:"sdk"`
}

func main() {
	// Wait for scrapi to be ready
	for i := 0; i < 10; i++ {
		resp, err := http.Get(SCRAPI_URL + "/ready")
		if err == nil && resp.StatusCode == 200 {
			resp.Body.Close()
			break
		}
		if resp != nil {
			resp.Body.Close()
		}

		fmt.Println("Waiting for scrapi to be ready")
		time.Sleep(1 * time.Second)
	}

	// Initialize Statsig
	wrapper := Initialize()

	// Run update every second
	for {
		update(&wrapper)
		time.Sleep(1 * time.Second)
	}
}

func readSdkState() SdkState {
	data, err := os.ReadFile("/shared-volume/state.json")
	if err != nil {
		panic(err)
	}

	var state State
	if err := json.Unmarshal(data, &state); err != nil {
		panic(err)
	}

	return state.SDK
}

func profile(name string, userID string, extra string, qps int, fn func()) {
	var durations []float64

	for i := 0; i < qps; i++ {
		start := time.Now()
		fn()
		end := time.Now()
		duration := float64(end.Sub(start).Microseconds()) / 1000.0
		durations = append(durations, duration)
	}

	result := map[string]interface{}{
		"name":   name,
		"userID": userID,
		"extra":  extra,
		"qps":    qps,
	}

	if qps > 0 {
		sort.Float64s(durations)
		median := durations[len(durations)/2]
		p99 := durations[int(float64(len(durations))*0.99)]
		min := durations[0]
		max := durations[len(durations)-1]

		result["median"] = median
		result["p99"] = p99
		result["min"] = min
		result["max"] = max

		fmt.Printf("%s took %.2fms (p99), %.2fms (max)\n", name, p99, max)
	}

	PROFILE_ARR = append(PROFILE_ARR, result)
}

func update(wrapper *StatsigWrapper) {
	fmt.Println("--------------------------------------- [ Update ]")

	state := readSdkState()
	PROFILE_ARR = nil

	fmt.Printf("Users: %d\n", len(state.Users))
	fmt.Printf("Gates: count(%d) qps(%d)\n", len(state.Gate.Names), state.Gate.QPS)
	fmt.Printf("Events: count(%d) qps(%d)\n", len(state.LogEvent.Events), state.LogEvent.QPS)

	for _, userData := range state.Users {
		wrapper.SetUser(userData.UserID)

		for _, gateName := range state.Gate.Names {
			profile(
				"check_gate",
				userData.UserID,
				gateName,
				state.Gate.QPS,
				func() { wrapper.CheckGate(gateName) },
			)
		}

		for _, event := range state.LogEvent.Events {
			profile(
				"log_event",
				userData.UserID,
				event.EventName,
				state.LogEvent.QPS,
				func() { wrapper.LogEvent(event.EventName) },
			)
		}

		profile(
			"gcir",
			userData.UserID,
			"",
			state.GCIR.QPS,
			func() { wrapper.GetClientInitializeResponse() },
		)
	}

	writeProfileData(wrapper)
}

func writeProfileData(wrapper *StatsigWrapper) {
	data, err := json.MarshalIndent(PROFILE_ARR, "", "  ")
	if err != nil {
		panic(err)
	}

	variant := "legacy"
	if wrapper.isCore {
		variant = "core"
	}

	slug := fmt.Sprintf("profile-go-%s", variant)
	tempFile := fmt.Sprintf("/shared-volume/%s-temp.json", slug)
	finalFile := fmt.Sprintf("/shared-volume/%s.json", slug)

	if err := os.WriteFile(tempFile, data, 0644); err != nil {
		panic(err)
	}

	cmd := exec.Command("mv", tempFile, finalFile)
	if err := cmd.Run(); err != nil {
		panic(err)
	}
}
