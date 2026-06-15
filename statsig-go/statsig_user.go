package statsig_go_core

import (
	"encoding/json"
	"fmt"
	"runtime"
)

type StatsigUser struct {
	ref uint64
}

// todo: introduce custom type for handling valid JSON primitives only instead of using 'any'
type StatsigUserBuilder struct {
	UserID string `json:"userID"`
	// map[string] string | number
	CustomIDs  map[string]any `json:"customIDs"`
	Email      *string        `json:"email"`
	IpAddress  *string        `json:"ip"`
	UserAgent  *string        `json:"userAgent"`
	Country    *string        `json:"country"`
	Locale     *string        `json:"locale"`
	AppVersion *string        `json:"appVersion"`
	// map[string] string | number | boolean | array<string>
	Custom *map[string]any `json:"custom"`
	// map[string] string | number | boolean | array<string>
	PrivateAttributes *map[string]any `json:"privateAttributes"`

	// nilUserID, set via WithoutUserID, serializes the user with no userID field
	// at all (a true null/absent userID at the FFI boundary, distinct from an
	// empty-string userID). When false, Build serializes UserID as-is ("" when
	// unset) — the default, backward-compatible wire shape.
	nilUserID bool
}

func NewUserBuilderWithUserID(userID string) *StatsigUserBuilder {
	return &StatsigUserBuilder{
		UserID: userID,
	}
}

func NewUserBuilderWithCustomIDs(customIDs map[string]any) *StatsigUserBuilder {
	return &StatsigUserBuilder{
		CustomIDs: customIDs,
	}
}

func (b *StatsigUserBuilder) WithUserID(userID string) *StatsigUserBuilder {
	b.UserID = userID
	b.nilUserID = false
	return b
}

// WithoutUserID builds a user with no userID: the serialized payload omits the
// userID field entirely, representing a true null/absent userID to statsig-rust
// (distinct from an empty-string userID). Use this for users identified solely by
// custom IDs.
//
// This is a short-term, non-breaking addition. The long-term direction is to type
// UserID as *string so null is representable directly; WithoutUserID will be
// retired in favor of a nil UserID when that lands.
func (b *StatsigUserBuilder) WithoutUserID() *StatsigUserBuilder {
	// nilUserID alone drives omission in marshalUserJSON; UserID is left untouched.
	b.nilUserID = true
	return b
}

func (b *StatsigUserBuilder) WithCustomIDs(customIDs map[string]any) *StatsigUserBuilder {
	b.CustomIDs = customIDs
	return b
}

func (b *StatsigUserBuilder) WithEmail(email string) *StatsigUserBuilder {
	b.Email = &email
	return b
}

func (b *StatsigUserBuilder) WithIpAddress(ipAddress string) *StatsigUserBuilder {
	b.IpAddress = &ipAddress
	return b
}

func (b *StatsigUserBuilder) WithUserAgent(userAgent string) *StatsigUserBuilder {
	b.UserAgent = &userAgent
	return b
}

func (b *StatsigUserBuilder) WithCountry(country string) *StatsigUserBuilder {
	b.Country = &country
	return b
}

func (b *StatsigUserBuilder) WithLocale(locale string) *StatsigUserBuilder {
	b.Locale = &locale
	return b
}

func (b *StatsigUserBuilder) WithAppVersion(appVersion string) *StatsigUserBuilder {
	b.AppVersion = &appVersion
	return b
}

func (b *StatsigUserBuilder) WithCustom(custom map[string]any) *StatsigUserBuilder {
	b.Custom = &custom
	return b
}

func (b *StatsigUserBuilder) WithPrivateAttributes(privateAttributes map[string]any) *StatsigUserBuilder {
	b.PrivateAttributes = &privateAttributes
	return b
}

// marshalUserJSON renders the wire JSON sent to statsig-rust without mutating the
// builder. By default an unset UserID serializes as "" (the legacy wire shape);
// when WithoutUserID was called, the userID field is omitted entirely.
//
// The alias type drops StatsigUserBuilder's methods while keeping its json tags;
// the outer UserID *string shadows alias.UserID (a shallower field wins in
// encoding/json), giving a single userID key with omit-on-nil behavior. All other
// fields serialize from the embedded value, so new builder fields need no change
// here.
func (b *StatsigUserBuilder) marshalUserJSON() ([]byte, error) {
	type alias StatsigUserBuilder
	wire := struct {
		*alias
		UserID *string `json:"userID,omitempty"`
	}{alias: (*alias)(b)}

	if !b.nilUserID {
		wire.UserID = &b.UserID
	}

	return json.Marshal(wire)
}

func (b *StatsigUserBuilder) Build() (*StatsigUser, error) {
	jsonData, err := b.marshalUserJSON()
	if err != nil {
		return nil, fmt.Errorf("error marshalling user: %v", err)
	}

	userRef := GetFFI().statsig_user_create_from_data(
		string(jsonData),
	)

	if userRef == 0 {
		return nil, fmt.Errorf("error creating StatsigUser")
	}

	user := &StatsigUser{
		ref: userRef,
	}

	runtime.SetFinalizer(user, func(obj *StatsigUser) {
		GetFFI().statsig_user_release(obj.ref)
	})

	return user, nil
}
