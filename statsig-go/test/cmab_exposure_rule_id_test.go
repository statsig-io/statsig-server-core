package test

import (
	"os"
	"strings"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

// Go uses the raw experiment path (statsig_get_raw_experiment), so these
// cover the automatic exposure enqueued from a raw CMAB evaluation.

func setupCmabTest(t *testing.T) (*statsig_go.Statsig, *MockScrapi) {
	resData, err := os.ReadFile("../../statsig-rust/tests/data/dcs_cmab_exposure_rule_id.json")
	if err != nil {
		t.Fatalf("error reading fixture: %v", err)
	}

	statsig, scrapi, _ := SetupTestWithDcsData(t, resData)
	t.Cleanup(scrapi.Close)
	return statsig, scrapi
}

func getLoggedCmabRuleID(t *testing.T, scrapi *MockScrapi) string {
	var ruleIDs []string
	for _, event := range scrapi.Events() {
		if event["eventName"] != "statsig::config_exposure" {
			continue
		}
		metadata, _ := event["metadata"].(map[string]any)
		ruleID, _ := metadata["ruleID"].(string)
		ruleIDs = append(ruleIDs, ruleID)
	}

	if len(ruleIDs) != 1 {
		t.Fatalf("expected 1 config exposure, got %d: %v", len(ruleIDs), ruleIDs)
	}
	return ruleIDs[0]
}

func TestCmabExposurePreservesExploreSuffix(t *testing.T) {
	statsig, scrapi := setupCmabTest(t)

	user, _ := statsig_go.NewUserBuilderWithUserID("raw-explore-user").Build()
	experiment := statsig.GetExperiment(user, "cmab_always_explore")
	statsig.Shutdown()

	if !strings.HasSuffix(experiment.RuleID, ":explore") {
		t.Errorf("expected returned rule ID to end with ':explore', got '%s'", experiment.RuleID)
	}

	if logged := getLoggedCmabRuleID(t, scrapi); logged != experiment.RuleID {
		t.Errorf("expected logged rule ID '%s', got '%s'", experiment.RuleID, logged)
	}
}

func TestCmabExposureExploitRuleIDRemainsBare(t *testing.T) {
	statsig, scrapi := setupCmabTest(t)

	user, _ := statsig_go.NewUserBuilderWithUserID("raw-exploit-user").Build()
	experiment := statsig.GetExperiment(user, "cmab_never_explore")
	statsig.Shutdown()

	if experiment.RuleID != "treatment-group-id" {
		t.Errorf("expected returned rule ID 'treatment-group-id', got '%s'", experiment.RuleID)
	}

	if logged := getLoggedCmabRuleID(t, scrapi); logged != experiment.RuleID {
		t.Errorf("expected logged rule ID '%s', got '%s'", experiment.RuleID, logged)
	}
}
