package main

/*
#cgo LDFLAGS: -L../target/aarch64-macos/release -lstatsig_ffi
#cgo CFLAGS: -I../statsig-ffi/include
#include "statsig_ffi.h"
#include <stdlib.h>
*/
import "C"
import (
	"fmt"
	"unsafe"
)

func main() {
	// Create a new Statsig instance
	sdkKey := C.CString("your-sdk-key")
	defer C.free(unsafe.Pointer(sdkKey))
	
	options := C.CString("{}")
	defer C.free(unsafe.Pointer(options))
	
	statsigRef := C.statsig_create(sdkKey, options)
	if statsigRef == nil {
		fmt.Println("Failed to create Statsig instance")
		return
	}
	defer C.statsig_release(statsigRef)
	
	// Initialize Statsig
	C.statsig_initialize_blocking(statsigRef)
	
	// Create a user
	userJson := C.CString(`{"userID": "test-user"}`)
	defer C.free(unsafe.Pointer(userJson))
	
	// Check a feature gate
	gateName := C.CString("test_gate")
	defer C.free(unsafe.Pointer(gateName))
	
	result := C.statsig_check_gate(statsigRef, userJson, gateName, nil)
	fmt.Printf("Feature gate result: %v\n", result)
	
	// Shutdown Statsig
	C.statsig_shutdown_blocking(statsigRef)
}
