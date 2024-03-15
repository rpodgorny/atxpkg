package main

import (
	"testing"

	"github.com/gookit/goutil/fsutil"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestInstallPackage(t *testing.T) {
	tmpDir := t.TempDir()
	installedPackage := lo.Must(InstallPackage(
		"./test_data/atx300-base-6.3-1.atxpkg.zip",
		tmpDir,
		false,
	))
	assert.Equal(t, "6.3-1", installedPackage.Version)
	assert.FileExists(t, tmpDir+"/atx300/memsh.mem")
	assert.NoFileExists(t, tmpDir+"/.atxpkg_backup")
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
	assert.Equal(t, "6.3-1", installedPackage.Version)
	assert.FileExists(t, tmpDir+"/atx300/memsh.mem")
	assert.NoFileExists(t, tmpDir+"/.atxpkg_backup")
}

func TestUpdatePackageWithConflict(t *testing.T) {
	tmpDir := t.TempDir()
	installedPackage := lo.Must(InstallPackage(
		"./test_data/test-1.0-1.atxpkg.zip",
		tmpDir,
		false,
	))
	assert.Equal(t, "1.0-1", installedPackage.Version)
	lo.Must0(fsutil.WriteFile(tmpDir+"/test/new", []byte("x\n"), 0o644))
	_, err := UpdatePackage(
		"./test_data/test-2.0-1.atxpkg.zip",
		"test",
		*installedPackage,
		tmpDir,
		false,
	)
	assert.Error(t, err)
}

func TestUpdatePackageWithBackup(t *testing.T) {
	tmpDir := t.TempDir()
	installedPackage := lo.Must(InstallPackage(
		"./test_data/test-1.0-1.atxpkg.zip",
		tmpDir,
		false,
	))
	assert.Equal(t, "1.0-1", installedPackage.Version)
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
	assert.Equal(t, "2.0-1", installedPackage.Version)
	assert.FileExists(t, tmpDir+"/test/protected1.atxpkg_new")
	assert.NoFileExists(t, tmpDir+"/test/protected2.atxpkg_new")
	assert.NoFileExists(t, tmpDir+"/test/protected3.atxpkg_new")
	assert.NoFileExists(t, tmpDir+"/test/unprotected.atxpkg_new")
}
