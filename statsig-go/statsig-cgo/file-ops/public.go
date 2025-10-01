package fileOps

import "os"

type FileOps interface {
	DownloadFile(url, outputPath string) error
	UnzipBinary(zipFilePath string, outputDir string) error
	Mkdir(path string, perm os.FileMode, recursive bool) bool
}

var internal FileOps = fileOps{}

func GetFileOps() FileOps {
	return internal
}
