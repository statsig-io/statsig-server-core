package main

import (
	"encoding/json"
	"fmt"
	"os"
)

func main() {
	sdkVariant := os.Getenv("SDK_VARIANT")

	if sdkVariant == "core" {
		BenchCore()
	} else {
		BenchLegacy()
	}
}

func writeResults(results *[]BenchmarkResult, sdkType string, sdkVersion string) {
	root := map[string]interface{}{
		"sdkType":    sdkType,
		"sdkVersion": sdkVersion,
		"results":    results,
	}

	jsonData, err := json.MarshalIndent(root, "", "  ")
	if err != nil {
		panic(fmt.Sprintf("Failed to marshal results: %v", err))
	}

	outPath := fmt.Sprintf("/shared-volume/%s-%s-results.json", sdkType, sdkVersion)
	if err := os.WriteFile(outPath, jsonData, 0644); err != nil {
		panic(fmt.Sprintf("Failed to write results: %v", err))
	}
}
