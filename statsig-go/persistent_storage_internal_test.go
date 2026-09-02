package statsig_go_core

import (
	"testing"
	"unsafe"
)

// The native side reads the load result with CStr::from_ptr, which scans for a
// terminating NUL, so the buffer we hand back must carry one of its own rather
// than relying on whatever happens to follow the marshalled JSON in the Go heap.
func TestRetainLoadBufferIsNulTerminated(t *testing.T) {
	storage := &PersistentStorage{}

	data := []byte(`{"a_config":{"value":true}}`)
	ptr := storage.retainLoadBuffer(data)

	buffer := unsafe.Slice(ptr, len(data)+1)
	if string(buffer[:len(data)]) != string(data) {
		t.Errorf("expected buffer to hold %q, got %q", data, buffer[:len(data)])
	}

	if buffer[len(data)] != 0 {
		t.Errorf("expected buffer to be NUL terminated, got %q", buffer[len(data)])
	}
}

// The native side copies out of the buffer after the callback has returned, so
// the buffer has to stay reachable from Go until well past that point.
func TestRetainLoadBufferKeepsRecentBuffersReachable(t *testing.T) {
	storage := &PersistentStorage{}

	first := storage.retainLoadBuffer([]byte("first"))

	for i := 0; i < loadBufferRetention-1; i++ {
		storage.retainLoadBuffer([]byte("filler"))
	}

	retained := 0
	for _, buffer := range storage.loadBuffers {
		if len(buffer) > 0 && &buffer[0] == first {
			retained++
		}
	}

	if retained != 1 {
		t.Errorf("expected the first buffer to still be referenced after %d more loads, found %d references", loadBufferRetention-1, retained)
	}
}
