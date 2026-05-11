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

func TestStatsigUserBuilder_AllowNilUserID(t *testing.T) {
	cases := []struct {
		name      string
		allowNil  bool
		configure func(b *statsig_go.StatsigUserBuilder) *statsig_go.StatsigUserBuilder
	}{
		{
			name:      "default_with_user_id",
			allowNil:  false,
			configure: func(b *statsig_go.StatsigUserBuilder) *statsig_go.StatsigUserBuilder { return b.WithUserID("u1") },
		},
		{
			name:      "default_with_empty_user_id",
			allowNil:  false,
			configure: func(b *statsig_go.StatsigUserBuilder) *statsig_go.StatsigUserBuilder { return b.WithUserID("") },
		},
		{
			name:      "default_with_nil_user_id_custom_ids",
			allowNil:  false,
			configure: func(b *statsig_go.StatsigUserBuilder) *statsig_go.StatsigUserBuilder { return b },
		},
		{
			name:      "optin_with_user_id",
			allowNil:  true,
			configure: func(b *statsig_go.StatsigUserBuilder) *statsig_go.StatsigUserBuilder { return b.WithUserID("u1") },
		},
		{
			name:      "optin_with_empty_user_id",
			allowNil:  true,
			configure: func(b *statsig_go.StatsigUserBuilder) *statsig_go.StatsigUserBuilder { return b.WithUserID("") },
		},
		{
			name:      "optin_with_nil_user_id_custom_ids",
			allowNil:  true,
			configure: func(b *statsig_go.StatsigUserBuilder) *statsig_go.StatsigUserBuilder { return b },
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			s := newStatsigForUserBuilder(t, tc.allowNil)
			b := tc.configure(s.NewUserBuilderWithCustomIDs(map[string]any{"stableID": "x"}))

			user, err := b.Build()
			if err != nil {
				t.Fatalf("Build() error = %v", err)
			}
			if user == nil {
				t.Fatalf("Build() returned nil user with no error")
			}
		})
	}
}

func TestStatsigUserBuilder_AllowNilUserID_FreeFunctionDefault(t *testing.T) {
	cases := []struct {
		name    string
		builder func() *statsig_go.StatsigUserBuilder
	}{
		{
			name:    "free_fn_with_user_id",
			builder: func() *statsig_go.StatsigUserBuilder { return statsig_go.NewUserBuilderWithUserID("u1") },
		},
		{
			name:    "free_fn_custom_ids_only_coerces_nil",
			builder: func() *statsig_go.StatsigUserBuilder { return statsig_go.NewUserBuilderWithCustomIDs(map[string]any{"stableID": "x"}) },
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			user, err := tc.builder().Build()
			if err != nil {
				t.Fatalf("Build() error = %v", err)
			}
			if user == nil {
				t.Fatalf("Build() returned nil user with no error")
			}
		})
	}
}

func TestStatsigUserBuilder_BuildDoesNotMutate(t *testing.T) {
	s := newStatsigForUserBuilder(t, false)
	b := s.NewUserBuilderWithCustomIDs(map[string]any{"stableID": "x"})

	if b.UserID != nil {
		t.Fatalf("precondition: builder.UserID = %v, want nil", b.UserID)
	}

	if _, err := b.Build(); err != nil {
		t.Fatalf("first Build() error = %v", err)
	}
	if b.UserID != nil {
		t.Fatalf("after first Build(): builder.UserID = %v, want nil (Build must not mutate)", *b.UserID)
	}

	if _, err := b.Build(); err != nil {
		t.Fatalf("second Build() error = %v", err)
	}
	if b.UserID != nil {
		t.Fatalf("after second Build(): builder.UserID = %v, want nil (Build must not mutate)", *b.UserID)
	}
}

func newStatsigForUserBuilder(t *testing.T, allowNilUserID bool) *statsig_go.Statsig {
	t.Helper()
	opts, err := statsig_go.NewOptionsBuilder().
		WithAllowNilUserID(allowNilUserID).
		Build()
	if err != nil {
		t.Fatalf("options Build() error = %v", err)
	}
	s, err := statsig_go.NewStatsigWithOptions("secret-test", opts)
	if err != nil {
		t.Fatalf("NewStatsigWithOptions error = %v", err)
	}
	return s
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
