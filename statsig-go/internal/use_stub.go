//go:build !linux && !darwin

package internal

func GetLibData() []byte {
	return []byte{}
}
