//go:build darwin || linux
// +build darwin linux

package statsig

/*
#cgo CFLAGS: -I${SRCDIR}/resources
#cgo darwin LDFLAGS: -lstatsig_ffi
#cgo linux LDFLAGS: -lstatsig_ffi

#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
