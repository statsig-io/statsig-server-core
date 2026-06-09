package statsig_go_core

import (
	"encoding/json"
	"testing"
)

// TestMarshalUserJSON_WireShape pins the serialization contract: by default the
// userID field is always present ("" when unset, byte-compatible with the legacy
// wire shape), and WithoutUserID omits the key entirely so statsig-rust receives a
// true null/absent userID.
func TestMarshalUserJSON_WireShape(t *testing.T) {
	cases := []struct {
		name          string
		build         func() *StatsigUserBuilder
		wantUserIDKey bool
		wantUserIDVal string
	}{
		{
			name:          "unset_user_id",
			build:         func() *StatsigUserBuilder { return NewUserBuilderWithCustomIDs(map[string]any{"stableID": "s1"}) },
			wantUserIDKey: true,
			wantUserIDVal: "",
		},
		{
			name:          "with_user_id",
			build:         func() *StatsigUserBuilder { return NewUserBuilderWithUserID("u1") },
			wantUserIDKey: true,
			wantUserIDVal: "u1",
		},
		{
			name:          "with_empty_user_id",
			build:         func() *StatsigUserBuilder { return NewUserBuilderWithUserID("") },
			wantUserIDKey: true,
			wantUserIDVal: "",
		},
		{
			name: "without_user_id_omits_key",
			build: func() *StatsigUserBuilder {
				return NewUserBuilderWithCustomIDs(map[string]any{"stableID": "s1"}).WithoutUserID()
			},
			wantUserIDKey: false,
		},
		{
			name:          "without_then_with_user_id_keeps_value",
			build:         func() *StatsigUserBuilder { return NewUserBuilderWithCustomIDs(nil).WithoutUserID().WithUserID("u2") },
			wantUserIDKey: true,
			wantUserIDVal: "u2",
		},
		{
			name:          "with_then_without_user_id_omits_key",
			build:         func() *StatsigUserBuilder { return NewUserBuilderWithUserID("u1").WithoutUserID() },
			wantUserIDKey: false,
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			data, err := tc.build().marshalUserJSON()
			if err != nil {
				t.Fatalf("marshalUserJSON() error = %v", err)
			}

			var m map[string]json.RawMessage
			if err := json.Unmarshal(data, &m); err != nil {
				t.Fatalf("unmarshal failed: %v (json=%s)", err, data)
			}

			raw, ok := m["userID"]
			if ok != tc.wantUserIDKey {
				t.Fatalf("userID key present = %v, want %v (json=%s)", ok, tc.wantUserIDKey, data)
			}
			if !ok {
				return
			}

			var got string
			if err := json.Unmarshal(raw, &got); err != nil {
				t.Fatalf("userID not a JSON string: %v (raw=%s)", err, raw)
			}
			if got != tc.wantUserIDVal {
				t.Errorf("userID = %q, want %q", got, tc.wantUserIDVal)
			}
		})
	}
}

// TestMarshalUserJSON_DoesNotMutate confirms marshalling leaves the builder
// untouched, so repeated Build() calls are stable.
func TestMarshalUserJSON_DoesNotMutate(t *testing.T) {
	b := NewUserBuilderWithUserID("u1")
	if _, err := b.marshalUserJSON(); err != nil {
		t.Fatalf("marshalUserJSON() error = %v", err)
	}
	if b.UserID != "u1" {
		t.Errorf("UserID mutated to %q, want %q", b.UserID, "u1")
	}
	if b.nilUserID {
		t.Errorf("nilUserID mutated to true")
	}
}
