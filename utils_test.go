package main

import (
	"fmt"
	"os"
	"slices"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestYesNo(t *testing.T) {
	tests := map[string]struct {
		msg  string
		def  string
		want bool
	}{
		"yes": {
			msg:  "ahoj",
			def:  "y",
			want: true,
		},
		"no": {
			msg:  "ahoj",
			def:  "n",
			want: false,
		},
		// TODO: this loops forever
		/*"nothing": {
			msg: "ahoj",
			def: "",
			want: false,
		},*/
	}
	for name, tt := range tests {
		t.Run(name, func(t *testing.T) {
			ans := YesNo(tt.msg, tt.def)
			assert.Equal(t, tt.want, ans)
		})
	}
}

func TestGetRecursiveListing(t *testing.T) {
	tmpDir := t.TempDir()
	err := os.MkdirAll(tmpDir+"/some/directory", os.ModePerm)
	assert.NoError(t, err)
	_, err = os.Create(tmpDir + "/some/file1")
	assert.NoError(t, err)
	_, err = os.Create(tmpDir + "/some/directory/file2")
	assert.NoError(t, err)
	dirs, files, err := GetRecursiveListing(tmpDir)
	assert.NoError(t, err)
	assert.Equal(t, []string{"some", "some/directory"}, dirs)
	slices.Sort(files)
	slices.Reverse(files)
	assert.Equal(t, []string{"some/file1", "some/directory/file2"}, files)
}

func TestGetAvailablePackages(t *testing.T) {
	// TODO: make it work offline
	packages := GetAvailablePackages(
		[]string{
			"http://atxpkg.asterix.cz",
			"http://atxpkg-dev.asterix.cz",
			"./test_data",
		},
		false,
	)
	// TODO: this does not test anything
	fmt.Printf("%v", packages)
}
