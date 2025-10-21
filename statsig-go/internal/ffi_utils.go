package internal

import "unsafe"

func GoStringFromPointer(inputPtr *byte, inputLength uint64) *string {
	if inputPtr == nil {
		return nil
	}

	s := string(unsafe.Slice(inputPtr, inputLength))
	return &s
}

func UnperformantGoStringFromPointer(inputPtr *byte) *string {
	if inputPtr == nil {
		return nil
	}

	var n uintptr
	for {
		if *(*byte)(unsafe.Add(unsafe.Pointer(inputPtr), n)) == 0 {
			break
		}
		n++
	}

	s := string(unsafe.Slice(inputPtr, n))
	return &s
}
