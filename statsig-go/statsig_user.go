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

func (b *StatsigUserBuilder) Build() (*StatsigUser, error) {
	jsonData, err := json.Marshal(b)
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
