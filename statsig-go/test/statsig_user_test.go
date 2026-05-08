package test

import (
	"encoding/json"
	"testing"

	statsig_go "github.com/statsig-io/statsig-go-core"
)

func TestStatsigUserBuilder(t *testing.T) {
	_, err := statsig_go.NewUserBuilderWithUserID("user-id").Build()

	if err != nil {
		t.Errorf("error creating StatsigUser: %v", err)
	}
}

func TestStatsigUserBuilderJSONShape(t *testing.T) {
	cases := []struct {
		name           string
		builder        *statsig_go.StatsigUserBuilder
		wantUserIDKey  bool
		wantUserIDVal  string
	}{
		{
			name:          "with_user_id",
			builder:       statsig_go.NewUserBuilderWithUserID("u1"),
			wantUserIDKey: true,
			wantUserIDVal: "u1",
		},
		{
			name:          "with_empty_user_id",
			builder:       statsig_go.NewUserBuilderWithUserID(""),
			wantUserIDKey: true,
			wantUserIDVal: "",
		},
		{
			name:          "custom_ids_only",
			builder:       statsig_go.NewUserBuilderWithCustomIDs(map[string]any{"stableID": "abc"}),
			wantUserIDKey: false,
		},
		{
			name:          "override_to_empty",
			builder:       statsig_go.NewUserBuilderWithUserID("u1").WithUserID(""),
			wantUserIDKey: true,
			wantUserIDVal: "",
		},
		{
			name:          "custom_ids_then_user_id",
			builder:       statsig_go.NewUserBuilderWithCustomIDs(map[string]any{"stableID": "abc"}).WithUserID("u2"),
			wantUserIDKey: true,
			wantUserIDVal: "u2",
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			data, err := json.Marshal(tc.builder)
			if err != nil {
				t.Fatalf("json.Marshal failed: %v", err)
			}

			var m map[string]json.RawMessage
			if err := json.Unmarshal(data, &m); err != nil {
				t.Fatalf("json.Unmarshal failed: %v", err)
			}

			raw, ok := m["userID"]
			if ok != tc.wantUserIDKey {
				t.Fatalf("userID key presence = %v, want %v (json=%s)", ok, tc.wantUserIDKey, string(data))
			}
			if !ok {
				return
			}

			var got string
			if err := json.Unmarshal(raw, &got); err != nil {
				t.Fatalf("userID value not a JSON string: %v (raw=%s)", err, string(raw))
			}
			if got != tc.wantUserIDVal {
				t.Errorf("userID = %q, want %q", got, tc.wantUserIDVal)
			}
		})
	}
}
