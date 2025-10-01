package test

import (
	"fmt"
	"os"
	"testing"

	statsig_go_core "github.com/statsig-io/statsig-go-core"
)

func TestStatsigGoCore(t *testing.T) {
	sdk_key := os.Getenv("test_api_key")
	if sdk_key == "" {
		t.Errorf("test_api_key is not set")
	}
	statsig := statsig_go_core.NewStatsig(sdk_key)
	statsig.Initialize()

	user := statsig_go_core.NewStatsigUser("user-id")
	gate := statsig.CheckGate(user, "test_public")
	fmt.Println("Gate ", gate)

	if !gate {
		t.Errorf("Gate test_public is false but should be true")
	}
}
