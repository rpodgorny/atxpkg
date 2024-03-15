package main

import (
	"testing"

	"github.com/gookit/goutil/fsutil"
	"github.com/samber/lo"
)

func TestInstallPackage(t *testing.T) {
	tmpDir := t.TempDir()
	installedPackage := lo.Must(InstallPackage(
		"./test_data/atx300-base-6.3-1.atxpkg.zip",
		tmpDir,
		false,
	))
	lo.Must0(installedPackage.Version == "6.3-1")
	lo.Must0(fsutil.FileExists(tmpDir + "/atx300/memsh.mem"))
	// TODO: add more tests
}

func TestUpdatePackage(t *testing.T) {
	tmpDir := t.TempDir()
	installedPackage := lo.Must(UpdatePackage(
		"./test_data/atx300-base-6.3-1.atxpkg.zip",
		"atx300-base",
		InstalledPackage{
			Version: "3.3",
			Md5sums: map[string]string{},
			Backup:  []string{},
		},
		tmpDir,
		false,
	))
	lo.Must0(installedPackage.Version == "6.3-1")
	lo.Must0(fsutil.FileExists(tmpDir + "/atx300/memsh.mem"))
	// TODO: add more tests
}

func TestUpdatePackageWithBackup(t *testing.T) {
	tmpDir := t.TempDir()
	installedPackage := lo.Must(InstallPackage(
		"./test_data/test-1.0-1.atxpkg.zip",
		tmpDir,
		false,
	))
	lo.Must0(installedPackage.Version == "1.0-1")
	lo.Must0(fsutil.WriteFile(tmpDir+"/test/protected1", []byte("x\n"), 0o644))
	lo.Must0(fsutil.WriteFile(tmpDir+"/test/protected2", []byte("2\n"), 0o644))
	lo.Must0(fsutil.WriteFile(tmpDir+"/test/unprotected", []byte("2\n"), 0o644))
	installedPackage = lo.Must(UpdatePackage(
		"./test_data/test-2.0-1.atxpkg.zip",
		"test",
		*installedPackage,
		tmpDir,
		false,
	))
	lo.Must0(installedPackage.Version == "2.0-1")
	lo.Must0(fsutil.FileExists(tmpDir + "/test/protected1"))
	lo.Must0(fsutil.FileExists(tmpDir + "/test/protected1.atxpkg_new"))
	lo.Must0(!fsutil.FileExists(tmpDir + "/test/protected2.atxpkg_new"))
	lo.Must0(!fsutil.FileExists(tmpDir + "/test/protected3.atxpkg_new"))
	lo.Must0(!fsutil.FileExists(tmpDir + "/test/unprotected.atxpkg_new"))
}
