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
		"./test_data/atx300-base-6.3-1.atxpkg.zip",
		tmpDir,
		false,
	))
	lo.Must0(installedPackage.Version == "6.3-1")
	lo.Must0(fsutil.WriteFile(tmpDir+"/atx300/set/base/base.ini", []byte("test"), 0o644)) // protected file
	lo.Must0(fsutil.WriteFile(tmpDir+"/atx300/memsh.mem", []byte("test"), 0o644))         // unprotected file
	installedPackage = lo.Must(UpdatePackage(
		"./test_data/atx300-base-6.3-1.atxpkg.zip",
		"atx300-base",
		*installedPackage,
		tmpDir,
		false,
	))
	lo.Must0(installedPackage.Version == "6.3-1")
	lo.Must0(fsutil.FileExists(tmpDir + "/atx300/set/base/base.ini"))
	lo.Must0(fsutil.FileExists(tmpDir + "/atx300/set/base/base.ini.atxpkg_new"))
	lo.Must0(fsutil.FileExists(tmpDir + "/atx300/memsh.mem"))
	lo.Must0(!fsutil.FileExists(tmpDir + "/atx300/memsh.mem.atxpkg_new"))
	// TODO: add more tests
}
