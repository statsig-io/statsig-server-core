package test

import (
	"os"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func TestStatsig(t *testing.T) {
	sdk_key := os.Getenv("test_api_key")
	if sdk_key == "" {
		t.Errorf("test_api_key is not set")
	}

	statsig := statsig_go.NewStatsig(sdk_key)
	statsig.Initialize()

	user, err := statsig_go.NewUserBuilderWithUserID("user-id").Build()
	if err != nil {
		t.Errorf("error creating StatsigUser: %v", err)
	}

	gate := statsig.CheckGate(user, "test_public")

	if !gate {
		t.Errorf("Gate test_public is false but should be true")
	}
}
