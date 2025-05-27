package statsig

/*
#cgo LDFLAGS: -L../target/aarch64-macos/debug -lstatsig_ffi
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
