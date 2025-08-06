package main

import (
    "os"
)

func main() {
	sdkVariant := os.Getenv("SDK_VARIANT")

	if sdkVariant == "core" {
		BenchCore()
	} else {
		BenchLegacy()
	}
}
