package tests

import (
	"testing"

	statsig "github.com/statsig-io/statsig-server-core/statsig-go/src"
)

func TestGetParameterStoreBasic(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("9").
		WithAppVersion("1.3").
		WithUserAgent("Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1").
		WithIpAddress("1.0.0.0").
		WithLocale("en_US").
		Build()

	_, _, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "", nil)

	psOptions := statsig.ParameterStoreOptions{DisableExposureLogging: false}

	defer teardown()

	paramStoreName := "gatedValueParamsStore"

	paramStore := s.GetParameterStore(*user, paramStoreName, &psOptions)

	stringVal := paramStore.GetString("string_gated_value_param", "")
	if stringVal != "fail" {
		t.Errorf("Expected string_gated_value_param to be 'fail' but got %v", stringVal)
	}

	boolVal := paramStore.GetBoolean("boolean_gated_value_param", false)
	if boolVal != true {
		t.Errorf("Expected boolean_gated_value_param to be true but got %v", boolVal)
	}

	numberVal := paramStore.GetInt("number_gated_value_param", 0)
	if numberVal != 10 {
		t.Errorf("Expected number_gated_value_param to be 10 but got %v", numberVal)
	}

	objectVal := paramStore.GetMap("object_gated_value_param", map[string]interface{}{})
	if objectVal["key"] != "value" {
		t.Errorf("Expected object_gated_value_param[\"key\"] to be 'value' but got %v", objectVal["key"])
	}

	arrayVal := paramStore.GetInterface("array_gated_value_param", []interface{}{})
	if len(arrayVal) != 0 {
		t.Errorf("Expected array_gated_value_param to be empty but got %v", arrayVal)
	}

}

func TestGetParameterStoreOptionsLogging(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("9").
		WithAppVersion("1.3").
		WithUserAgent("Mozilla/5.0 (iPhone; CPU iPhone OS 10_3_1 like Mac OS X) AppleWebKit/603.1.30 (KHTML, like Gecko) Version/10.0 Mobile/14E304 Safari/602.1").
		WithIpAddress("1.0.0.0").
		WithLocale("en_US").
		Build()

	_, scrapiServer, s, teardown := setupStatsigTest(t, "eval_proj_dcs.json", "", nil)

	psOptions := statsig.ParameterStoreOptions{DisableExposureLogging: false}

	defer teardown()

	paramStoreName := "experimentParamsStore"

	paramStore := s.GetParameterStore(*user, paramStoreName, &psOptions)

	_ = paramStore.GetString("string_exp_param", "")

	s.FlushEvents()

	if !checkEventNameExists(scrapiServer.fetchLoggedEvents(), "statsig::config_exposure") {
		t.Errorf("Error occurred, config exposure event was not logged")
	}

}
