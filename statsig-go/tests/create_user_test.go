package tests

import (
	"testing"
	"time"

	"github.com/statsig-io/private-statsig-server-core/statsig-go"
)

func TestCreateUser(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("test-user0").
		WithEmail("test@test.com").
		WithIpAddress("127.0.0.1").
		WithUserAgent("test-user-agent").
		WithCountry("US").
		WithLocale("en-US").
		WithAppVersion("1.0.0").
		WithCustom(map[string]interface{}{
			"feature_enabled":  true,
			"experiment_group": "beta_group_3",
		}).
		WithPrivateAttributes(map[string]interface{}{
			"app_build_number": 204,
			"nested": map[string]interface{}{
				"sub_key": "nested_value",
			},
		}).
		WithStatsigEnvironment(map[string]string{
			"appVersion": "2.3.1",
		}).
		WithCustomIds(map[string]string{
			"stable_id": "user-stable-9876",
		}).
		Build()

	if user.UserID != "test-user0" {
		t.Errorf("Expected UserID test-user0, but got %s", user.UserID)
	}
	if user.Email != "test@test.com" {
		t.Errorf("Expected Email test@test.com, but got %s", user.Email)
	}
	if user.IpAddress != "127.0.0.1" {
		t.Errorf("Expected IpAddress 127.0.0.1, but got %s", user.IpAddress)
	}
	if user.UserAgent != "test-user-agent" {
		t.Errorf("Expected UserAgent test-user-agent, but got %s", user.UserAgent)
	}
	if user.Country != "US" {
		t.Errorf("Expected Country US, but got %s", user.Country)
	}
	if user.Locale != "en-US" {
		t.Errorf("Expected Locale en-US, but got %s", user.Locale)
	}
	if user.AppVersion != "1.0.0" {
		t.Errorf("Expected AppVersion 1.0.0, but got %s", user.AppVersion)
	}
	if user.Custom["feature_enabled"] != true {
		t.Errorf("Expected Custom feature_enabled true, but got %v", user.Custom["feature_enabled"])
	}
	if user.Custom["experiment_group"] != "beta_group_3" {
		t.Errorf("Expected Custom experiment_group beta_group_3, but got %v", user.Custom["experiment_group"])
	}
	if user.PrivateAttributes["app_build_number"] != 204 {
		t.Errorf("Expected PrivateAttributes app_build_number 204, but got %v", user.PrivateAttributes["app_build_number"])
	}
	nestedVal := user.PrivateAttributes["nested"].(map[string]interface{})["sub_key"]
	if nestedVal != "nested_value" {
		t.Errorf("Expected nested.sub_key nested_value, but got %v", nestedVal)
	}
	if user.StatsigEnvironment["appVersion"] != "2.3.1" {
		t.Errorf("Expected StatsigEnvironment appVersion 2.3.1, but got %v", user.StatsigEnvironment["appVersion"])
	}
	if user.CustomIDs["stable_id"] != "user-stable-9876" {
		t.Errorf("Expected CustomIDs stable_id user-stable-9876, but got %v", user.CustomIDs["stable_id"])
	}

	user = nil
	time.Sleep(1 * time.Second)

}

func TestCreatePartialUser(t *testing.T) {
	user := statsig.NewStatsigUserBuilder().
		WithUserID("test-user1").
		WithEmail("test-user1@gmail.com").
		WithCountry("USA").
		Build()

	if user.UserID != "test-user1" {
		t.Errorf("expected UserID to be 'test-user1', got %s", user.UserID)
	}
	if user.Email != "test-user1@gmail.com" {
		t.Errorf("expected Email to be test-user1@gmail.com, got %s", user.Email)
	}
	if user.Country != "USA" {
		t.Errorf("expected Country to be 'USA', got %s", user.Country)
	}
	// Check other fields to ensure they're zero values (not set)
	if user.IpAddress != "" {
		t.Errorf("expected IpAddress to be empty, got %s", user.IpAddress)
	}
	if user.UserAgent != "" {
		t.Errorf("expected UserAgent to be empty, got %s", user.UserAgent)
	}
	if user.Locale != "" {
		t.Errorf("expected Locale to be empty, got %s", user.Locale)
	}
	if user.AppVersion != "" {
		t.Errorf("expected AppVersion to be empty, got %s", user.AppVersion)
	}
	if user.Custom != nil {
		t.Errorf("expected Custom to be nil, got %v", user.Custom)
	}
	if user.PrivateAttributes != nil {
		t.Errorf("expected PrivateAttributes to be nil, got %v", user.PrivateAttributes)
	}
	if user.StatsigEnvironment != nil {
		t.Errorf("expected StatsigEnvironment to be nil, got %v", user.StatsigEnvironment)
	}
	if user.CustomIDs != nil {
		t.Errorf("expected CustomIDs to be nil, got %v", user.CustomIDs)
	}

}
