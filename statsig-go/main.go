package statsig

/*
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
import (
	"fmt"
	"runtime"
)

type User struct {
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
	innerRef           *C.char
}

func (u *User) GetInnerRef() *C.char {
	return u.innerRef
}

func NewStatsigUser(userID string, email string, ipAddress string, userAgent string, country string, locale string, appVersion string, custom map[string]interface{}, privateAttributes map[string]interface{}, statsigEnvironment map[string]string, customIDs map[string]string) *User {
	userRef := C.statsig_user_create(C.CString(userID), C.CString(email), C.CString(ipAddress), C.CString(userAgent), C.CString(country), C.CString(locale), C.CString(appVersion), C.CString(""), C.CString(""), C.CString(""))

	u := &User{
		innerRef:           userRef,
		UserID:             userID,
		Email:              email,
		IpAddress:          ipAddress,
		UserAgent:          userAgent,
		Country:            country,
		Locale:             locale,
		AppVersion:         appVersion,
		Custom:             custom,
		PrivateAttributes:  privateAttributes,
		StatsigEnvironment: statsigEnvironment,
		CustomIDs:          customIDs,
	}

	// Set finalizer on the Go object
	runtime.SetFinalizer(u, func(obj *User) {
		fmt.Println("Cleaning up user:", obj.UserID)
		C.statsig_user_release(obj.innerRef)
	})

	return u
}
