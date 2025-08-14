package main

import (
	"fmt"
	"os"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
)

func verify() {

	sdkKey := os.Getenv("STATSIG_SERVER_SDK_KEY")
	if sdkKey == "" {
		fmt.Println("STATSIG_SERVER_SDK_KEY is not set")
		os.Exit(1)
	}

	s, err := statsig.NewStatsig(sdkKey, statsig.StatsigOptions{})
	if err != nil {
		fmt.Printf("Failed to create Statsig instance: %v\n", err)
		os.Exit(1)
	}

	_, err = s.Initialize()
	if err != nil {
		fmt.Printf("Failed to initialize Statsig: %v\n", err)
		os.Exit(1)
	}

	user := statsig.NewStatsigUserBuilder().
		WithUserID("user-id").
		Build()

	gate := s.CheckGate(*user, "test_public", nil)
	gcir := s.GetClientInitializeResponse(*user, nil)

	fmt.Println("-------------------------------- Get Client Initialize Response --------------------------------")
	fmt.Println(gcir)
	fmt.Println("-------------------------------------------------------------------------------------------------")

	fmt.Println("Gate test_public", gate)

	if !gate {
		fmt.Println("Gate test_public is false but should be true")
		os.Exit(1)
	}

	if gcir == "" {
		fmt.Println("GCIR is missing required fields")
		os.Exit(1)
	}

	fmt.Println("All checks passsed, shutting down...")
	s.Shutdown()
	fmt.Println("Shutdown complete")
}

func main() {
	verify()
}
