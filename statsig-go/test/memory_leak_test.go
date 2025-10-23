package test

import (
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"runtime"
	"strconv"
	"strings"
	"testing"
	"time"
)

func getRssBytes(t *testing.T) int64 {
	pid := os.Getpid()

	lines, err := exec.Command("ps", "-o", "rss=", "-p", strconv.Itoa(pid)).Output()
	if err != nil {
		t.Fatalf("Error getting RSS: %v", err)
	}

	rss := strings.TrimSpace(string(lines))
	bytes, err := strconv.ParseInt(rss, 10, 64)
	if err != nil {
		t.Fatalf("Error parsing RSS: %v", err)
	}

	return bytes * 1024
}

func TestMemoryLeak(t *testing.T) {
	resData := loadLargeDcsData(t)
	statsig, _, user := SetupTestWithDcsData(t, resData)

	time.Sleep(1 * time.Second)

	for range 10 {
		_ = statsig.GetFeatureGate(user, "test_public")
		_ = statsig.GetDynamicConfig(user, "test_empty_array")
		_ = statsig.GetExperiment(user, "exp_with_obj_and_array")
		_ = statsig.GetLayer(user, "layer_with_many_params")
		_ = statsig.GetClientInitResponse(user)
	}

	time.Sleep(1 * time.Second)

	triggerGC()

	initialRss := getRssBytes(t)
	fmt.Println("Initial RSS: ", humanizeBytes(initialRss))

	for range 100 {
		_ = statsig.GetFeatureGate(user, "test_public")
		_ = statsig.GetDynamicConfig(user, "test_empty_array")
		_ = statsig.GetExperiment(user, "exp_with_obj_and_array")
		_ = statsig.GetLayer(user, "layer_with_many_params")
		_ = statsig.GetClientInitResponse(user)
	}

	time.Sleep(1 * time.Second)

	triggerGC()

	finalRss := getRssBytes(t)
	fmt.Println("Final RSS: ", humanizeBytes(finalRss))

	percentChange := float64(finalRss-initialRss) / float64(initialRss) * 100
	delta := finalRss - initialRss

	if percentChange > 10 {
		t.Errorf("Memory leak detected: %s (%.2f%%)", humanizeBytes(delta), percentChange)
	} else {
		fmt.Printf("Memory change within acceptable range: %s (%.2f%%)", humanizeBytes(delta), percentChange)
	}
}

func triggerGC() {
	for range 5 {
		runtime.GC()
		time.Sleep(100 * time.Millisecond)
	}
}

func loadLargeDcsData(t *testing.T) []byte {
	resData, err := os.ReadFile("../../statsig-rust/tests/data/eval_proj_dcs.json")
	if err != nil {
		t.Fatalf("error reading file: %v", err)
	}

	resString := string(resData)
	largeString := strings.Repeat("a", 2000000) + "b"

	// use regex to find and replace all "header_text" fields with largeString
	re := regexp.MustCompile(`"header_text": "([^"]+)"`)
	resString = re.ReplaceAllString(resString, `"header_text": "`+largeString+`"`)

	return []byte(resString)
}

func humanizeBytes(bytes int64) string {
	if bytes < 1024 {
		return fmt.Sprintf("%d B", bytes)
	}

	if bytes < 1024*1024 {
		return fmt.Sprintf("%.2f KB", float64(bytes)/1024)
	}

	if bytes < 1024*1024*1024 {
		return fmt.Sprintf("%.2f MB", float64(bytes)/(1024*1024))
	}

	return fmt.Sprintf("%.2f GB", float64(bytes)/(1024*1024*1024))
}
