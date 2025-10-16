package main

import (
	"fmt"
	"os"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func main() {
	sdkKey := os.Getenv("STATSIG_SERVER_SDK_KEY")
	if sdkKey == "" {
		fmt.Println("STATSIG_SERVER_SDK_KEY is not set")
		os.Exit(1)
	}

	statsig, err := statsig_go.NewStatsig(sdkKey)
	if err != nil {
		fmt.Println("Failed to create Statsig instance: ", err)
		os.Exit(1)
	}

	statsig.Initialize()

	user, err := statsig_go.NewUserBuilderWithUserID("a_user").Build()
	if err != nil {
		fmt.Println("Failed to create Statsig user: ", err)
		os.Exit(1)
	}

	gate := statsig.CheckGate(user, "test_public")
	gcir := statsig.GetClientInitResponse(user)

	fmt.Println("-------------------------------- Get Client Initialize Response --------------------------------")
	fmt.Println(*gcir)
	fmt.Println("-------------------------------------------------------------------------------------------------")

	fmt.Println("Gate test_public: ", gate)

	// statsig.Shutdown()
}
