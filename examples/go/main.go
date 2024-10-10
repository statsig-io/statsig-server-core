package main

import (
	"fmt"
	"time"
	"os"

	"statsig.com/sdk/statsig"
)

func main() {
	name := "Dan"
	email := "dan.smith@example.com"

	user := statsig.NewUser(name, email)
	defer user.Destroy()

	sdkKey := os.Getenv("test_api_key")
	statsigInstance := statsig.NewStatsig(user, sdkKey)
	defer statsigInstance.Destroy()

	// gateName := "test_public"

	// Uncomment the following lines to check a gate
	// result := statsigInstance.CheckGate(gateName)
	// fmt.Printf("Gate check %s!\n", if result "passed" else "failed")

	start := time.Now()
	var result string
	for i := 0; i < 1000; i++ {
		result = statsigInstance.GetClientInitResponse()
		// Uncomment the following lines to check different gates
		// result = statsigInstance.CheckGate("test_50_50")
		// result = statsigInstance.CheckGate("double_nested_gates")
		// result = statsigInstance.CheckGate("test_public")
	}
	duration := time.Since(start)

	// Print the last result from GetClientInitResponse
	fmt.Println(result)
	fmt.Printf("Duration: %v ms\n", duration.Milliseconds())
}
