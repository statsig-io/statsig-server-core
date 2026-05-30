package internal

import "unsafe"

// GoStringFromPointer copies a Rust-owned, NUL-unaware byte buffer of the given
// length into a Go string. inputPtr is the raw address of memory allocated by
// the Rust FFI (not the Go heap), carried as a uintptr so the Go GC / stack
// mover never tracks or relocates it. The uintptr->unsafe.Pointer conversion is
// safe because the buffer is owned by Rust and stays valid until the caller
// frees it; vet's unsafeptr warning does not apply to foreign memory.
func GoStringFromPointer(inputPtr uintptr, inputLength uint64) *string {
	if inputPtr == 0 {
		return nil
	}

	s := string(unsafe.Slice((*byte)(unsafe.Pointer(inputPtr)), inputLength)) //nolint:govet // foreign (Rust-owned) memory, see doc comment
	return &s
}

func UnperformantGoStringFromPointer(inputPtr uintptr) *string {
	if inputPtr == 0 {
		return nil
	}

	var n uintptr
	for {
		if *(*byte)(unsafe.Pointer(inputPtr + n)) == 0 { //nolint:govet // foreign (Rust-owned) memory, see GoStringFromPointer
			break
		}
		n++
	}

	s := string(unsafe.Slice((*byte)(unsafe.Pointer(inputPtr)), n)) //nolint:govet // foreign (Rust-owned) memory, see GoStringFromPointer
	return &s
}
