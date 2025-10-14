//go:build darwin

package internal

import (
	bin "github.com/statsig-io/go-server-core-binaries-macos"
)

func GetLibData() []byte {
	return bin.GetBinaryData()
}
