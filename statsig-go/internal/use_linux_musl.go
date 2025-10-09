//go:build linux && musl

package internal

import (
	bin "github.com/statsig-io/go-server-core-binaries-linux-musl"
)

func GetLibData() []byte {
	return bin.GetBinaryData()
}
