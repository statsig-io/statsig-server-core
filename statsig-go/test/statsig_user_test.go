package test

import (
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func TestStatsigUserBuilder(t *testing.T) {
	_, err := statsig_go.NewUserBuilderWithUserID("user-id").Build()

	if err != nil {
		t.Errorf("error creating StatsigUser: %v", err)
	}
}
