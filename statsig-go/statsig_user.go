package statsig_go_core

import (
	"encoding/json"
	"fmt"
)

type StatsigUser struct {
	ref uint64
}

type StatsigUserBuilder struct {
	UserID            string
	CustomIDs         map[string]string
	Email             *string
	IpAddress         *string
	UserAgent         *string
	Country           *string
	Locale            *string
	AppVersion        *string
	Custom            *map[string]string
	PrivateAttributes *map[string]string
}

func NewUserBuilderWithUserID(userID string) *StatsigUserBuilder {
	return &StatsigUserBuilder{
		UserID: userID,
	}
}

func NewUserBuilderWithCustomIDs(customIDs map[string]string) *StatsigUserBuilder {
	return &StatsigUserBuilder{
		CustomIDs: customIDs,
	}
}

func (b *StatsigUserBuilder) WithUserID(userID string) *StatsigUserBuilder {
	b.UserID = userID
	return b
}

func (b *StatsigUserBuilder) WithCustomIDs(customIDs map[string]string) *StatsigUserBuilder {
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

func (b *StatsigUserBuilder) WithCustom(custom map[string]string) *StatsigUserBuilder {
	b.Custom = &custom
	return b
}

func (b *StatsigUserBuilder) WithPrivateAttributes(privateAttributes map[string]string) *StatsigUserBuilder {
	b.PrivateAttributes = &privateAttributes
	return b
}

func (b *StatsigUserBuilder) Build() (*StatsigUser, error) {
	customIDsJSON, err := toJsonString(&b.CustomIDs)
	if err != nil {
		return nil, fmt.Errorf("error marshalling user.customIDs: %v", err)
	}

	customJSON, err := toJsonString(b.Custom)
	if err != nil {
		return nil, fmt.Errorf("error marshalling user.custom: %v", err)
	}

	privateAttributesJSON, err := toJsonString(b.PrivateAttributes)
	if err != nil {
		return nil, fmt.Errorf("error marshalling user.privateAttributes: %v", err)
	}

	userRef := GetFFI().statsig_user_create(
		b.UserID,
		string(customIDsJSON),
		b.Email, b.IpAddress, b.UserAgent, b.Country, b.Locale, b.AppVersion,
		&customJSON, &privateAttributesJSON,
	)

	if userRef == 0 {
		return nil, fmt.Errorf("error creating StatsigUser")
	}

	user := &StatsigUser{
		ref: userRef,
	}

	return user, nil
}

func toJsonString(data *map[string]string) (string, error) {
	if data == nil {
		return "{}", nil
	}

	jsonStr, err := json.Marshal(data)
	if err != nil {
		return "{}", err
	}

	return string(jsonStr), nil
}
