//go:build linux && !musl

package internal

import (
	bin "github.com/statsig-io/go-server-core-binaries-linux-gnu"
)

func GetLibData() []byte {
	return bin.GetBinaryData()
}
