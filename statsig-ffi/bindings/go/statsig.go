package statsig

/*
#cgo LDFLAGS: -L../../../target/release -lstatsig_ffi
#cgo CFLAGS: -I../../../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
import (
	"unsafe"
)

type User struct {
	ptr *C.User
}

type Statsig struct {
	ptr *C.Statsig
}

// NewUser creates a new user.
func NewUser(name, email string) *User {
	cname := C.CString(name)
	cemail := C.CString(email)
	defer C.free(unsafe.Pointer(cname))
	defer C.free(unsafe.Pointer(cemail))
	return &User{ptr: C.create_user(cname, cemail)}
}

// Destroy destroys the user.
func (u *User) Destroy() {
	if u.ptr != nil {
		C.destroy_user(u.ptr)
		u.ptr = nil
	}
}

func NewStatsig(user *User, sdkKey string) *Statsig {
	csdkKey := C.CString(sdkKey)
	defer C.free(unsafe.Pointer(csdkKey))
	return &Statsig{ptr: C.create_statsig_for_user(user.ptr, csdkKey)}
}

func (s *Statsig) Destroy() {
	if s.ptr != nil {
		C.destroy_statsig(s.ptr)
		s.ptr = nil
	}
}

func (s *Statsig) CheckGate(gateName string) bool {
	cgateName := C.CString(gateName)
	defer C.free(unsafe.Pointer(cgateName))
	return C.check_gate(s.ptr, cgateName) != 0
}

func (s *Statsig) GetClientInitResponse() string {
	return C.GoString(C.statsig_get_client_init_response(s.ptr))
}
