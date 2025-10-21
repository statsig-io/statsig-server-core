//go:build linux

package internal

import (
	"os/exec"
	"strings"

	gnu "github.com/statsig-io/go-server-core-binaries-linux-gnu"
	musl "github.com/statsig-io/go-server-core-binaries-linux-musl"
)

func GetLibData() []byte {
	if isMusl() {
		return musl.GetBinaryData()
	}

	return gnu.GetBinaryData()
}

func isMusl() bool {
	out, err := exec.Command("ldd", "--version").CombinedOutput()
	if err == nil && strings.Contains(string(out), "musl") {
		return true
	}

	out, err = exec.Command("ldd", "/bin/ls").CombinedOutput()
	return err == nil && strings.Contains(string(out), "musl")
}
