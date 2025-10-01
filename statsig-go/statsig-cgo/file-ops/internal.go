package fileOps

import (
	"archive/zip"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
)

type fileOps struct{}

func validateUrl(url string) error {
	trustedUrl := "https://github.com/statsig-io/statsig-server-core/releases/download/"
	if !strings.HasPrefix(url, trustedUrl) {
		return fmt.Errorf("untrusted URL")
	}
	return nil
}

func createSafeFile(baseDir, filename string, allowlist map[string]bool, allowedPrefixes []string) (*os.File, error) {
	cleanName := filepath.Base(filename)
	fullPath := filepath.Join(baseDir, cleanName)

	absBase, err := filepath.Abs(baseDir)
	if err != nil {
		return nil, fmt.Errorf("could not resolve base dir: %w", err)
	}
	absTarget, err := filepath.Abs(fullPath)
	if err != nil {
		return nil, fmt.Errorf("could not resolve target file: %w", err)
	}
	if !strings.HasPrefix(absTarget, absBase+string(os.PathSeparator)) {
		return nil, fmt.Errorf("path traversal attempt detected: %s", absTarget)
	}

	// Optionally enforce allowlist
	if allowlist != nil && !allowlist[cleanName] && !hasAllowedPrefix(cleanName, allowedPrefixes) {
		return nil, fmt.Errorf("filename %q is not allowed (allowlist: %v, prefixes: %v)", cleanName, allowlist, allowedPrefixes)
	}

	return os.Create(absTarget)
}

func hasAllowedPrefix(name string, prefixes []string) bool {
	for _, prefix := range prefixes {
		if strings.HasPrefix(name, prefix) {
			return true
		}
	}
	return false
}

func (f fileOps) DownloadFile(url, outputPath string) error {
	if err := validateUrl(url); err != nil {
		return fmt.Errorf("could not download file: %w", err)
	}

	fmt.Println("Downloading file from:", url)

	resp, err := http.Get(url)
	if err != nil {
		return fmt.Errorf("failed to download: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return fmt.Errorf("failed to download: %s", resp.Status)
	}

	baseDir := filepath.Dir(outputPath)
	allowlist := map[string]bool{
		"libstatsig_ffi.so":    true,
		"libstatsig_ffi.dylib": true,
		"statsig_ffi.dll":      true,
		"statsig-ffi-":         true,
		"statsig_ffi.h":        true,
	}
	baseName := filepath.Base(outputPath)
	outFile, err := createSafeFile(baseDir, baseName, allowlist, []string{"statsig-ffi"})
	if err != nil {
		return fmt.Errorf("failed to create file safely: %w", err)
	}
	defer outFile.Close()

	if _, err := io.Copy(outFile, resp.Body); err != nil {
		return fmt.Errorf("failed to write file: %w", err)
	}

	return nil
}

func (f fileOps) Mkdir(path string, perm os.FileMode, recursive bool) bool {
	if recursive {
		err := os.MkdirAll(path, perm)
		return err == nil
	}
	err := os.Mkdir(path, perm)
	return err == nil
}

// openTrustedZip ensures the ZIP file path itself is trusted before opening
func openTrustedZip(zipFilePath string, trustedDir string) (*zip.ReadCloser, error) {
	absZip, err := filepath.Abs(zipFilePath)
	if err != nil {
		return nil, fmt.Errorf("could not resolve zip path: %w", err)
	}
	absTrusted, err := filepath.Abs(trustedDir)
	if err != nil {
		return nil, fmt.Errorf("could not resolve trusted dir: %w", err)
	}
	if !strings.HasPrefix(absZip, absTrusted+string(os.PathSeparator)) {
		return nil, fmt.Errorf("zip file is not in trusted directory: %s", zipFilePath)
	}
	return zip.OpenReader(absZip)
}

func (f fileOps) UnzipBinary(zipFilePath string, outputDir string) error {
	fmt.Println("\n-- Unzipping Statsig FFI Binary --")
	fmt.Printf(" Input Path: %s\n", zipFilePath)

	if !f.Mkdir(outputDir, 0755, true) {
		return fmt.Errorf("failed to create output directory: %s", outputDir)
	}

	// Validate the ZIP file path before opening (this is what Aikido wants)
	reader, err := openTrustedZip(zipFilePath, filepath.Dir(zipFilePath))
	if err != nil {
		return err
	}
	defer reader.Close()

	allowlist := map[string]bool{
		"libstatsig_ffi.so":    true,
		"libstatsig_ffi.dylib": true,
		"statsig_ffi.dll":      true,
		"statsig_ffi.h":        true,
		"statsig-ffi-":         true,
	}

	absBase, err := filepath.Abs(outputDir)
	if err != nil {
		return fmt.Errorf("failed to resolve output directory: %w", err)
	}

	for _, file := range reader.File {
		destPath := filepath.Join(outputDir, file.Name)
		absDest, err := filepath.Abs(destPath)
		if err != nil {
			return err
		}
		if !strings.HasPrefix(absDest, absBase+string(os.PathSeparator)) {
			return fmt.Errorf("illegal file path: %s", file.Name)
		}

		if file.FileInfo().IsDir() {
			if err := os.MkdirAll(absDest, file.Mode()); err != nil {
				return err
			}
			continue
		}

		outFile, err := createSafeFile(outputDir, file.Name, allowlist, []string{})
		if err != nil {
			return fmt.Errorf("failed to create file safely: %w", err)
		}

		srcFile, err := file.Open()
		if err != nil {
			outFile.Close()
			return err
		}

		if _, err := io.Copy(outFile, srcFile); err != nil {
			srcFile.Close()
			outFile.Close()
			return err
		}

		srcFile.Close()
		outFile.Close()
		fmt.Printf(" Extracted: %s\n", filepath.Join(outputDir, filepath.Base(file.Name)))
	}

	fmt.Println("-----------------------------------")
	return nil
}
