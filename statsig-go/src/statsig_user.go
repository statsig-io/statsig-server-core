package statsig

/*
#cgo CFLAGS: -I../../statsig-ffi/include
#include "statsig_ffi.h"

#include <stdlib.h>
*/
import "C"
import (
	"runtime"

	"github.com/statsig-io/statsig-server-core/statsig-go/src/utils"
)

type StatsigUser struct {
	UserID             string
	Email              string
	IpAddress          string
	UserAgent          string
	Country            string
	Locale             string
	AppVersion         string
	Custom             map[string]interface{}
	PrivateAttributes  map[string]interface{}
	StatsigEnvironment map[string]string
	CustomIDs          map[string]string
	innerRef           uint64
}

type StatsigUserBuilder struct {
	user StatsigUser
}

// TODO(varshaa): remove this function
func (u *StatsigUser) GetInnerRef() uint64 {
	return u.innerRef
}

func (u *StatsigUserBuilder) Build() *StatsigUser {
	userRef := C.statsig_user_create(
		C.CString(u.user.UserID),
		C.CString(utils.ConvertJSONToString(u.user.CustomIDs)),
		C.CString(u.user.Email),
		C.CString(u.user.IpAddress),
		C.CString(u.user.UserAgent),
		C.CString(u.user.Country),
		C.CString(u.user.Locale),
		C.CString(u.user.AppVersion),
		C.CString(utils.ConvertJSONToString(u.user.Custom)),
		C.CString(utils.ConvertJSONToString(u.user.PrivateAttributes)),
	)

	u.user.innerRef = uint64(userRef)

	user := &u.user

	// Set finalizer on the Go object
	runtime.SetFinalizer(user, func(obj *StatsigUser) {
		C.statsig_user_release(C.uint64_t(obj.innerRef))
	})

	return &u.user
}

func NewStatsigUserBuilder() *StatsigUserBuilder {
	return &StatsigUserBuilder{}
}

func (u *StatsigUserBuilder) WithUserID(userId string) *StatsigUserBuilder {
	u.user.UserID = userId
	return u
}

func (u *StatsigUserBuilder) WithEmail(email string) *StatsigUserBuilder {
	u.user.Email = email
	return u
}

func (u *StatsigUserBuilder) WithIpAddress(ipAddress string) *StatsigUserBuilder {
	u.user.IpAddress = ipAddress
	return u
}

func (u *StatsigUserBuilder) WithUserAgent(userAgent string) *StatsigUserBuilder {
	u.user.UserAgent = userAgent
	return u
}

func (u *StatsigUserBuilder) WithCountry(country string) *StatsigUserBuilder {
	u.user.Country = country
	return u
}

func (u *StatsigUserBuilder) WithLocale(locale string) *StatsigUserBuilder {
	u.user.Locale = locale
	return u
}

func (u *StatsigUserBuilder) WithAppVersion(appVersion string) *StatsigUserBuilder {
	u.user.AppVersion = appVersion
	return u
}

func (u *StatsigUserBuilder) WithCustom(custom map[string]interface{}) *StatsigUserBuilder {
	u.user.Custom = custom
	return u
}

func (u *StatsigUserBuilder) WithPrivateAttributes(privateAttributes map[string]interface{}) *StatsigUserBuilder {
	u.user.PrivateAttributes = privateAttributes
	return u
}

func (u *StatsigUserBuilder) WithStatsigEnvironment(statsigEnvironment map[string]string) *StatsigUserBuilder {
	u.user.StatsigEnvironment = statsigEnvironment
	return u
}

func (u *StatsigUserBuilder) WithCustomIds(customIds map[string]string) *StatsigUserBuilder {
	u.user.CustomIDs = customIds
	return u
}
