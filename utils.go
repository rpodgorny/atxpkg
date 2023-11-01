package main

import (
	"archive/zip"
	"bufio"
	"crypto/md5"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"io/fs"
	"log"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"slices"
	"strconv"
	"strings"
	"time"

	"github.com/pkg/errors"
	"github.com/samber/lo"
	"github.com/schollz/progressbar/v3"
)

// TODO: version parsing and comparison is cut-n-pasted from atxtools - unite!

// SplitVer splits version string to slice of integers
func SplitVer(ver string) []int {
	regex := regexp.MustCompile(`[.-]`)
	parts := regex.Split(ver, -1)
	var result []int
	for _, part := range parts {
		num, err := strconv.Atoi(part)
		if err == nil {
			result = append(result, num)
		}
	}
	return result
}

func CompareSlices(slice1, slice2 []int) int {
	minLen := len(slice1)
	if len(slice2) < minLen {
		minLen = len(slice2)
	}
	for i := 0; i < minLen; i++ {
		if slice1[i] < slice2[i] {
			return -1
		} else if slice1[i] > slice2[i] {
			return 1
		}
	}
	if len(slice1) < len(slice2) {
		return -1
	} else if len(slice1) > len(slice2) {
		return 1
	}
	return 0
}

func CompareVersions(v1, v2 string) int {
	return CompareSlices(SplitVer(v1), SplitVer(v2))
}

// TODO: find some function for this in "lo"
func UnwrapOrEmpty[T any](res T, err error) T {
	var empty T
	if err != nil {
		return empty
	}
	return res
}

func isUrl(s string) bool {
	return strings.HasPrefix(s, "http://") || strings.HasPrefix(s, "https://")
}

func FileExists(fn string) bool {
	fi, err := os.Stat(fn)
	if os.IsNotExist(err) {
		return false
	}
	return !fi.IsDir()
}

func DirExists(path string) bool {
	fi, err := os.Stat(path)
	if os.IsNotExist(err) {
		return false
	}
	return fi.IsDir()
}

func isEmptyDir(path string) (bool, error) {
	dir, err := os.Open(path)
	if err != nil {
		return false, err
	}
	defer dir.Close()

	_, err = dir.Readdir(1) // Try to read just one entry
	if err == nil {
		// Directory is not empty
		return false, nil
	}

	// If ReadDir returns an error indicating no more files, the directory is empty
	if err == io.EOF {
		return true, nil
	}

	// Some other error occurred while reading the directory
	return false, err
}

func readLines(fn string) ([]string, error) {
	f, err := os.Open(fn)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	var lines []string
	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	if err := scanner.Err(); err != nil {
		return nil, err
	}
	return lines, nil
}

func copyFile(fromFn string, toFn string) error {
	src, err := os.Open(fromFn)
	if err != nil {
		return err
	}
	defer src.Close()

	dst, err := os.Create(toFn)
	if err != nil {
		return err
	}
	defer dst.Close()

	if _, err := io.Copy(dst, src); err != nil {
		return err
	}

	fi, err := os.Stat(fromFn)
	if err != nil {
		return err
	}
	if err := os.Chmod(toFn, fi.Mode()); err != nil {
		return err
	}
	if err := os.Chtimes(toFn, fi.ModTime(), fi.ModTime()); err != nil {
		return err
	}

	return nil
}

func GetRepos(fn string) ([]string, error) {
	lines, err := readLines(fn)
	if err != nil {
		return nil, err
	}
	lines = lo.Map(lines, func(x string, _ int) string {
		return strings.TrimSpace(x)
	})
	lines = lo.Filter(lines, func(x string, _ int) bool {
		return !(x == "" || x[0] == '#' || x[0] == ';')
	})
	return lines, nil
}

func GetAvailablePackages(repos []string, offline bool) map[string][]string {
	ret := map[string][]string{}
	for _, repo := range repos {
		if offline && isUrl(repo) {
			continue
		}
		packageURLs := getRepoListing(repo)
		for _, packageURL := range packageURLs {
			packageFn := GetPackageFn(packageURL)
			if !isValidPackageFn(packageFn) {
				log.Printf("%s not a valid package filename\n", packageFn)
				continue
			}
			packageName := GetPackageName(packageFn)
			if urls, ok := ret[packageName]; ok {
				ret[packageName] = append(urls, packageURL)
			} else {
				ret[packageName] = []string{packageURL}
			}
		}
	}
	return ret
}

func getRepoListingHttp(url string) ([]string, error) {
	resp, err := http.Get(url)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	re := regexp.MustCompile(`[\w\-\._:/]+\.atxpkg\.\w+`)
	files := re.FindAllString(string(body), -1)

	result := lo.Map(files, func(x string, _ int) string {
		return url + "/" + x
	})
	return result, nil
}

func getRepoListingDir(path string) ([]string, error) {
	ret := []string{}
	err := filepath.Walk(path, func(path string, fi fs.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if fi.IsDir() {
			return nil
		}
		if !strings.HasSuffix(path, ".atxpkg.zip") {
			return nil
		}
		ret = append(ret, filepath.ToSlash(path))
		return nil
	})
	if err != nil {
		return nil, err
	}
	return ret, nil
}

func getRepoListing(repo string) []string {
	log.Printf("getting repo listing from %s\n", repo)

	if isUrl(repo) {
		res, err := getRepoListingHttp(repo)
		if err != nil {
			log.Printf("failed to get listing from %s: %v\n", repo, err)
			return nil
		}
		return res
	}

	ret, err := getRepoListingDir(repo)
	if err != nil {
		log.Printf("error accessing directory: %v\n", err)
		return nil
	}
	return ret
}

func DownloadPackageIfNeeded(url, cacheDir string) (string, error) {
	if !isUrl(url) {
		return url, nil
	}
	fn := filepath.Join(cacheDir, GetPackageFn(url))
	if FileExists(fn) {
		log.Printf("using cached %s\n", fn)
		return fn, nil
	}
	log.Printf("downloading %s to %s\n", url, fn)
	resumeFrom := 0
	if FileExists(fn+"_") {
		if resp, err := http.Head(url); err == nil {
			defer resp.Body.Close()
			if resp.StatusCode == http.StatusOK && resp.Header.Get("Accept-Ranges") == "bytes" {
				if fi, err := os.Stat(fn+"_"); err == nil {
					resumeFrom = int(fi.Size())
				}
			}
		}
	}

	f, err := os.OpenFile(fn+"_", os.O_WRONLY|os.O_CREATE|os.O_APPEND, 0644)
	if err != nil {
		return "", err
	}
	defer f.Close()

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return "", err
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resumeFrom > 0 {
		log.Printf("resuming from %v\n", resumeFrom)
		req.Header.Set("Range", fmt.Sprintf("bytes=%d-", resumeFrom))
	}

	bar := progressbar.DefaultBytes(resp.ContentLength, "")

	//if _, err := io.Copy(f, resp.Body); err != nil {
	if _, err := io.Copy(io.MultiWriter(f, bar), resp.Body); err != nil {
		return "", err
	}
	if err := os.Rename(fn+"_", fn); err != nil {
		return "", err
	}
	return fn, nil
}

func tryDelete(fn string) error {
	if _, err := os.Stat(fn); !os.IsNotExist(err) {
		delFn := fn + ".atxpkg_delete"
		for FileExists(delFn) {
			if err := os.Remove(delFn); err != nil {
				break
			}
			delFn += "_delete"
		}
		if err := os.Rename(fn, delFn); err != nil {
			return err
		}
		if err := os.Remove(delFn); err != nil {
			return err
		}
	}
	return nil
}

func InstallPackage(fn, prefix string, force bool) (*InstalledPackage, error) {
	name, versionNew := SplitPackageNameVersion(GetPackageFn(fn))
	log.Printf("installing %s-%s\n", name, versionNew)

	ret := &InstalledPackage{
		Version: versionNew,
		Md5sums: map[string]string{},
		Backup:  nil,
	}

	tmpDir, err := os.MkdirTemp("", "atxpkg")
	if err != nil {
		return nil, err
	}
	defer os.RemoveAll(tmpDir)

	if err := unzipTo(fn, tmpDir); err != nil {
		return nil, err
	}

	backupFn := filepath.Join(tmpDir, ".atxpkg_backup")
	if content, err := readLines(backupFn); err == nil {
		ret.Backup = content
	}

	dirs, files, err := GetRecursiveListing(tmpDir)
	if err != nil {
		return nil, err
	}
	files = lo.Filter(files, func(x string, _ int) bool {
		return !strings.HasPrefix(x, ".atxpkg_")
	})
	if !force {
		for _, fn := range files {
			if FileExists(prefix + "/" + fn) {
				return nil, errors.Errorf("file exists: %s", prefix+"/"+fn)
			}
		}
	}
	for _, d := range dirs {
		log.Printf("I %s\n", d)
		if err := os.MkdirAll(prefix+"/"+d, os.ModePerm); err != nil {
			return nil, err
		}
		srcInfo, err := os.Stat(tmpDir + "/" + d)
		if err != nil {
			return nil, err
		}
		if err := os.Chmod(prefix+"/"+d, srcInfo.Mode()); err != nil {
			return nil, err
		}
		if err := os.Chtimes(prefix+"/"+d, srcInfo.ModTime(), srcInfo.ModTime()); err != nil {
			return nil, err
		}
	}
	for _, f := range files {
		log.Printf("I %s\n", f)
		sum, err := GetMD5Sum(tmpDir + "/" + f)
		if err != nil {
			return nil, err
		}
		ret.Md5sums[f] = sum

		backupFn := filepath.Join(prefix, f)
		if FileExists(backupFn) && slices.Contains(ret.Backup, f) {
			log.Printf("saving untracked %s as %s.atxpkg_save\n", backupFn, backupFn)
			if err := os.Rename(backupFn, backupFn+".atxpkg_save"); err != nil {
				return nil, err
			}
		}
		targetFn := fmt.Sprintf("%s/%s", prefix, f)
		if err := tryDelete(targetFn); err != nil {
			return nil, err
		}
		if err := copyFile(tmpDir+"/"+f, targetFn); err != nil {
			return nil, err
		}
	}
	return ret, nil
}

func UpdatePackage(fn, nameOld string, installedPackage InstalledPackage, prefix string, force bool) (*InstalledPackage, error) {
	versionOld := installedPackage.Version
	name, versionNew := SplitPackageNameVersion(GetPackageFn(fn))
	log.Printf("updating %s-%s -> %s-%s\n", nameOld, versionOld, name, versionNew)

	ret := &InstalledPackage{
		Version: versionNew,
		Md5sums: map[string]string{},
		Backup:  nil,
	}

	tmpDir, err := os.MkdirTemp("", "atxpkg")
	if err != nil {
		return nil, err
	}
	defer os.RemoveAll(tmpDir)

	if err := unzipTo(fn, tmpDir); err != nil {
		return nil, err
	}

	backupFn := filepath.Join(tmpDir, ".atxpkg_backup")
	if content, err := readLines(backupFn); err == nil {
		ret.Backup = content
	}

	dirs, files, err := GetRecursiveListing(tmpDir)
	if err != nil {
		return nil, err
	}
	files = lo.Filter(files, func(x string, _ int) bool {
		return !strings.HasPrefix(x, ".atxpkg_")
	})

	for _, d := range dirs {
		targetDir := filepath.Join(prefix, d)
		log.Printf("I %s\n", targetDir)
		if DirExists(targetDir) {
			continue
		}
		if err := os.Mkdir(targetDir, os.ModePerm); err != nil {
			return nil, err
		}
	}

	if !force {
		for _, fn := range files {
			if FileExists(fn) {
				return nil, errors.Errorf("file exists: %v", fn)
			}
		}
	}

	for _, f := range files {
		sumNew, err := GetMD5Sum(filepath.Join(tmpDir, f))
		if err != nil {
			return nil, err
		}

		ret.Md5sums[f] = sumNew

		targetFn := filepath.Join(prefix, f)
		if FileExists(targetFn) {
			sumCurrent, err := GetMD5Sum(targetFn)
			if err != nil {
				return nil, err
			}

			if _, ok := installedPackage.Md5sums[f]; ok {
				if sumCurrent == sumNew {
					log.Printf("S %s\n", targetFn)
					continue
				}
			}

			if slices.Contains(ret.Backup, f) {
				log.Printf("saving changed %s as %s.atxpkg_save\n", targetFn, targetFn)
				if err := os.Rename(targetFn, targetFn+".atxpkg_save"); err != nil {
					return nil, err
				}
			}

			log.Printf("U %s\n", targetFn)
			if err := tryDelete(targetFn); err != nil {
				return nil, err
			}
		}

		if err := copyFile(filepath.Join(tmpDir, f), targetFn); err != nil {
			return nil, err
		}
	}

	// Remove files which are no longer in the new version
	filesToBackupOld := installedPackage.Backup
	for fn, md5sum := range installedPackage.Md5sums {
		if _, ok := ret.Md5sums[fn]; !ok {
			targetFn := filepath.Join(prefix, fn)
			if FileExists(targetFn) {
				sumCurrent, err := GetMD5Sum(targetFn)
				if err != nil {
					return nil, err
				}

				if !slices.Contains(lo.Keys(ret.Md5sums), fn) {
					if !slices.Contains(filesToBackupOld, fn) {
						log.Printf("DF %s\n", targetFn)
						if err := tryDelete(targetFn); err != nil {
							return nil, err
						}
					}
				} else {
					if sumCurrent != md5sum {
						log.Printf("saving changed %s as %s.atxpkg_save\n", targetFn, targetFn)
						if err := os.Rename(targetFn, targetFn+".atxpkg_save"); err != nil {
							return nil, err
						}
					}
				}

				dirName := filepath.Dir(targetFn)
				if dirName == prefix {
					continue
				}
				if empty, err := isEmptyDir(dirName); empty && err == nil {
					log.Printf("DD %s\n", dirName)
					if err := tryDelete(dirName); err != nil {
						return nil, err
					}
					dirName = filepath.Dir(dirName)
				}
			}
		}
	}

	return ret, nil
}

func RemovePackage(packageName string, installedPackages map[string]InstalledPackage, prefix string) error {
	version := installedPackages[packageName].Version
	log.Printf("removing package %s: %s\n", packageName, version)
	packageInfo := installedPackages[packageName]

	for fn, md5sum := range packageInfo.Md5sums {
		targetFn := filepath.Join(prefix, fn)
		if !FileExists(targetFn) {
			log.Printf("%s does not exist!\n", targetFn)
			continue
		}

		backup := false
		if slices.Contains(packageInfo.Backup, fn) {
			currentSum, err := GetMD5Sum(targetFn)
			if err != nil {
				log.Printf("Error calculating MD5 sum for %s: %v\n", targetFn, err)
			}
			if currentSum != md5sum {
				backup = true
			}
		}

		if backup {
			log.Printf("%s changed, saving as %s.atxpkg_backup\n", targetFn, targetFn)
			if err := os.Rename(targetFn, targetFn+".atxpkg_backup"); err != nil {
				return err
			}
		} else {
			log.Printf("DF %s\n", targetFn)
			if err := tryDelete(targetFn); err != nil {
				return err
			}
		}

		dirName := filepath.Dir(targetFn)
		if dirName == prefix {
			continue
		}
		if empty, err := isEmptyDir(dirName); empty && err == nil {
			log.Printf("DD %s\n", dirName)
			if err := tryDelete(dirName); err != nil {
				return err
			}
		}
	}
	return nil
}

func MergeConfigPackage(packageName string, installedPackages map[string]InstalledPackage, prefix string) error {
	packageInfo := installedPackages[packageName]
	filesToBackupOld := packageInfo.Backup

	for _, fn := range filesToBackupOld {
		for _, suffix := range []string{".atxpkg_backup", ".atxpkg_new", ".atxpkg_save"} {
			fnFull := filepath.Join(prefix, fn)
			fnFromFull := fnFull + suffix

			if _, err := os.Stat(fnFromFull); err == nil {
				log.Printf("found %s, running merge\n", fnFromFull)

				if err := merge(fnFull, fnFromFull); err != nil {
					return err
				}

				if YesNo("delete "+fnFromFull+"?", "n") {
					log.Printf("D %s\n", fnFromFull)
					if err := os.Remove(fnFromFull); err != nil {
						return err
					}
				}
			}
		}
	}
	return nil
}

func YesNo(s string, def string) bool {
	var q string
	if def == "y" {
		q = fmt.Sprintf("%s [Y/n] ", s)
	} else if def == "n" {
		q = fmt.Sprintf("%s [y/N] ", s)
	} else {
		q = fmt.Sprintf("%s [y/n] ", s)
	}
	reader := bufio.NewReader(os.Stdin)
	for {
		fmt.Print(q)
		ans, _ := reader.ReadString('\n')
		ans = strings.ToLower(strings.TrimSpace(ans))
		if ans == "y" {
			return true
		} else if ans == "n" {
			return false
		} else if ans == "" && def == "y" {
			return true
		} else if ans == "" && def == "n" {
			return false
		} else {
			fmt.Println("Invalid input. Please enter 'y' or 'n'.")
		}
	}
}

func merge(fn1, fn2 string) error {
	cmd := exec.Command("vim", "-d", fn1, fn2)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		return errors.Wrapf(err, "error running merge command")
	}
	return nil
}

func GetMD5Sum(fn string) (string, error) {
	file, err := os.Open(fn)
	if err != nil {
		return "", err
	}
	defer file.Close()

	hash := md5.New()
	if _, err := io.Copy(hash, file); err != nil {
		return "", err
	}
	hashInBytes := hash.Sum(nil)
	md5sum := hex.EncodeToString(hashInBytes)
	return md5sum, nil
}

func GetRecursiveListing(path string) (dirs []string, files []string, _ error) {
	var retDirs, retFiles []string
	err := filepath.Walk(path, func(filePath string, fi fs.FileInfo, err error) error {
		if err != nil {
			return err
		}
		relPath, err := filepath.Rel(path, filePath)
		if err != nil {
			return err
		}
		if relPath == "." {
			return nil
		}
		if fi.IsDir() {
			retDirs = append(retDirs, filepath.ToSlash(relPath))
		} else {
			retFiles = append(retFiles, filepath.ToSlash(relPath))
		}
		return nil
	})
	if err != nil {
		return nil, nil, err
	}
	return retDirs, retFiles, nil
}

type InstalledPackage struct {
	T       float64           `json:"t"`
	Version string            `json:"version"`
	Md5sums map[string]string `json:"md5sums"`
	Backup  []string          `json:"backup"`
}

func GetInstalledPackages(dbFn string) (map[string]InstalledPackage, error) {
	if _, err := os.Stat(dbFn); os.IsNotExist(err) {
		return nil, errors.Wrapf(err, "package database not found (%s)", dbFn)
	}

	f, err := os.Open(dbFn)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	var installedPackages map[string]InstalledPackage
	if err := json.NewDecoder(f).Decode(&installedPackages); err != nil {
		return nil, err
	}

	return installedPackages, nil
}

func SaveInstalledPackages(installedPackages map[string]InstalledPackage, dbFn string) error {
	f, err := os.Create(dbFn)
	if err != nil {
		return err
	}
	defer f.Close()

	encoder := json.NewEncoder(f)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(installedPackages); err != nil {
		return err
	}
	return nil
}

func GetSpecificVersionURL(urls []string, version string) string {
	for _, url := range urls {
		if GetPackageVersion(GetPackageFn(url)) == version {
			return url
		}
	}
	return ""
}

func CleanCache(cacheDir string) error {
	files, err := os.ReadDir(cacheDir)
	if err != nil {
		return errors.Wrapf(err, "error reading cache directory %s", cacheDir)
	}
	for _, file := range files {
		filePath := filepath.Join(cacheDir, file.Name())
		if err := os.Remove(filePath); err != nil {
			return errors.Wrapf(err, "error removing file %s", filePath)
		}
		log.Printf("D %s\n", filePath)
	}
	return nil
}

func GenFnToPackageNameMapping(installedPackages map[string]InstalledPackage, prefix string) map[string]string {
	ret := map[string]string{}
	for packageName, packageInfo := range installedPackages {
		md5sums := packageInfo.Md5sums
		for fn := range md5sums {
			ret[fmt.Sprintf("%s/%s", prefix, fn)] = packageName
		}
	}
	return ret
}

func GetMaxVersion(urls []string) string {
	var maxVersion string
	for _, url := range urls {
		packageVersion := GetPackageVersion(GetPackageFn(url))
		if CompareVersions(packageVersion, maxVersion) > 0 {
			maxVersion = packageVersion
		}
	}
	return maxVersion
}

func GetMaxVersionURL(urls []string) string {
	var maxVersionUrl string
	for _, url := range urls {
		packageVersion := GetPackageVersion(GetPackageFn(url))
		maxVersion := GetPackageVersion(GetPackageFn(maxVersionUrl))
		if CompareVersions(packageVersion, maxVersion) > 0 {
			maxVersionUrl = url
		}
	}
	return maxVersionUrl
}

func GetPackageFn(url string) string {
	parts := strings.Split(url, "/")
	return parts[len(parts)-1]
}

func SplitPackageNameVersion(pkgSpec string) (string, string) {
	re := regexp.MustCompile(`^(.+?)(?:-([\d.-]+))?(?:\.atxpkg\.zip)?$`)
	matches := re.FindStringSubmatch(pkgSpec)
	var name, version string
	if len(matches) >= 2 {
		name, version = matches[1], ""
		if len(matches) == 3 && matches[2] != "" {
			version = matches[2]
		}
	}
	return name, version
}

func GetPackageName(fn string) string {
	name, _ := SplitPackageNameVersion(fn)
	return name
}

func GetPackageVersion(fn string) string {
	_, version := SplitPackageNameVersion(fn)
	return version
}

func isValidPackageFn(fn string) bool {
	re := regexp.MustCompile(`[\w\-\.]+-[\d.]+-\d+\.atxpkg\.zip`)
	return re.MatchString(fn)
}

// TODO: unused?
func hasVersion(pkgSpec string) bool {
	re := regexp.MustCompile(`[\w\-\.]+-[\d.]+-\d+`)
	return re.MatchString(pkgSpec)
}

func unzipTo(fnZip string, path string) error {
	r, err := zip.OpenReader(fnZip)
	if err != nil {
		return err
	}
	defer r.Close()

	for _, file := range r.File {
		rc, err := file.Open()
		if err != nil {
			return err
		}
		defer rc.Close()

		filePath := filepath.Join(path, file.Name)
		fmt.Println("UNZIP", file.Name)
		if file.FileInfo().IsDir() {
			if err := os.Mkdir(filePath, os.ModePerm); err != nil {
				return err
			}
		} else {
			f, err := os.Create(filePath)
			if err != nil {
				return err
			}
			defer f.Close()

			if _, err := io.Copy(f, rc); err != nil {
				return err
			}
		}
		if err := os.Chmod(filePath, file.Mode()); err != nil {
			return err
		}
		if err := os.Chtimes(filePath, file.Modified, file.Modified); err != nil {
			return err
		}
	}
	return nil
}

func InstallPackages(
	packages []string,
	installedPackages map[string]InstalledPackage,
	prefix string,
	repos []string,
	force bool,
	offline bool,
	yes bool,
	no bool,
	downloadOnly bool,
	cacheDir string,
) (map[string]InstalledPackage, error) {
	availablePackages := GetAvailablePackages(repos, offline)

	for _, p := range packages {
		packageName := GetPackageName(p)
		if _, ok := installedPackages[packageName]; ok {
			if !force && !downloadOnly {
				return nil, errors.Errorf("package %s already installed", packageName)
			}
		}
		if _, ok := availablePackages[packageName]; !ok {
			return nil, errors.Errorf("unable to find url for package %s", packageName)
		}
	}

	urlsToInstall := []string{}
	for _, p := range packages {
		packageName, packageVersion := SplitPackageNameVersion(p)
		packageURLs, ok := availablePackages[packageName]
		if !ok {
			return nil, errors.Errorf("unable to find url for package %s", packageName)
		}
		var url string
		if packageVersion != "" {
			url = GetSpecificVersionURL(packageURLs, packageVersion)
		} else {
			url = GetMaxVersionURL(packageURLs)
		}
		urlsToInstall = append(urlsToInstall, url)
		packageName, packageVersion = SplitPackageNameVersion(GetPackageFn(url))
		fmt.Printf("install %v-%v\n", packageName, packageVersion)
	}

	if downloadOnly {
	} else if !no && !(yes || YesNo("continue?", "y")) {
		return installedPackages, nil
	}

	localFnsToInstall := []string{}
	for _, url := range urlsToInstall {
		localFn, err := DownloadPackageIfNeeded(url, cacheDir)
		if err != nil {
			return nil, errors.Wrapf(err, "failed to download %s or retrieve from %s", url, cacheDir)
		}
		localFnsToInstall = append(localFnsToInstall, localFn)
	}
	if downloadOnly {
		return installedPackages, nil
	}
	for _, localFn := range localFnsToInstall {
		packageName, packageVersion := SplitPackageNameVersion(GetPackageFn(localFn))
		packageInfo, err := InstallPackage(localFn, prefix, force)
		if err != nil {
			return nil, err
		}
		packageInfo.T = float64(time.Now().UnixNano()) / 1e9
		installedPackages[packageName] = *packageInfo
		fmt.Printf("%s-%s is now installed\n", packageName, packageVersion)
	}
	return installedPackages, nil
}

func UpdatePackages(
	packages []string,
	installedPackages map[string]InstalledPackage,
	prefix string,
	repos []string,
	force bool,
	offline bool,
	yes bool,
	no bool,
	downloadOnly bool,
	cacheDir string,
) (map[string]InstalledPackage, error) {
	type packageUpdate struct {
		nameOld    string
		versionOld string
		nameNew    string
		versionNew string
		url        string
		localFn    string
	}
	var packageUpdates []packageUpdate
	for _, p := range packages {
		var pu packageUpdate
		if strings.Contains(p, "..") {
			packageParts := strings.Split(p, "..")
			packageOld, packageNew := packageParts[0], packageParts[1]
			packageNameOld, packageVersionOld := SplitPackageNameVersion(packageOld)
			packageNameNew, packageVersionNew := SplitPackageNameVersion(packageNew)
			pu = packageUpdate{
				nameOld:    packageNameOld,
				versionOld: packageVersionOld,
				nameNew:    packageNameNew,
				versionNew: packageVersionNew,
			}
		} else {
			name, version := SplitPackageNameVersion(p)
			pu = packageUpdate{
				nameOld:    name,
				versionOld: "",
				nameNew:    name,
				versionNew: version,
			}
		}
		installedPackage, ok := installedPackages[pu.nameOld]
		if !ok {
			return nil, errors.Errorf("package %s not installed", pu.nameOld)
		}
		if pu.versionOld != "" {
			if pu.versionOld != installedPackage.Version {
				return nil, errors.Errorf("package %s-%s not installed", pu.nameOld, pu.versionOld)
			}
		} else {
			pu.versionOld = installedPackage.Version
		}
		if _, ok := installedPackages[pu.nameNew]; !ok {
			return nil, errors.Errorf("package %s already installed", pu.nameNew)
		}
		packageUpdates = append(packageUpdates, pu)
	}

	availablePackages := GetAvailablePackages(repos, offline)

	for i, pu := range packageUpdates {
		if _, ok := availablePackages[pu.nameNew]; !ok {
			return nil, errors.Errorf("package %s not available", pu.nameNew)
		}
		if pu.versionNew == "" {
			pu.versionNew = GetMaxVersion(availablePackages[pu.nameNew])                // TODO: this does not modify the original but still the following code depends on it - solve better
			packageUpdates[i].versionNew = GetMaxVersion(availablePackages[pu.nameNew]) // TODO: ugly shit
		}
		url := GetSpecificVersionURL(availablePackages[pu.nameNew], pu.versionNew)
		if url == "" {
			return nil, errors.Errorf("package %s-%s not available", pu.nameNew, pu.versionNew)
		}
		//pu.url = url
		packageUpdates[i].url = url // TODO: ugly shit

		if force || pu.versionNew != installedPackages[pu.nameOld].Version {
			fmt.Printf("update %s-%s -> %s-%s\n", pu.nameOld, pu.versionOld, pu.nameNew, pu.versionNew)
		}
	}

	if len(packageUpdates) == 0 {
		fmt.Println("nothing to update")
		return installedPackages, nil
	}

	if !no && !(yes || YesNo("continue?", "y")) {
		return installedPackages, nil
	}

	for i, pu := range packageUpdates {
		localFn, err := DownloadPackageIfNeeded(pu.url, cacheDir)
		if err != nil {
			return nil, errors.Wrapf(err, "failed to download %v or retrieve from %v", pu.url, cacheDir)
		}
		//pu.localFn = localFn
		packageUpdates[i].localFn = localFn // TODO: ugly shit
	}

	if downloadOnly {
		return installedPackages, nil
	}

	for _, pu := range packageUpdates {
		packageInfo, err := UpdatePackage(pu.localFn, pu.nameOld, installedPackages[pu.nameOld], prefix, force)
		if err != nil {
			return nil, errors.Wrapf(err, "failed to update package %v", pu.nameOld)
		}
		packageInfo.T = float64(time.Now().UnixNano()) / 1e9
		delete(installedPackages, pu.nameOld)
		installedPackages[pu.nameNew] = *packageInfo
		fmt.Printf("%s-%s updated to %s-%s\n", pu.nameOld, pu.versionOld, pu.nameNew, pu.versionNew)
	}
	return installedPackages, nil
}

func RemovePackages(
	packages []string,
	installedPackages map[string]InstalledPackage,
	prefix string,
	yes bool,
	no bool,
) (map[string]InstalledPackage, error) {
	for _, p := range packages {
		packageName, packageVersion := SplitPackageNameVersion(p)
		installedPackage, ok := installedPackages[packageName]
		if !ok {
			return nil, errors.Errorf("package %s not installed", packageName)
		}
		if packageVersion != "" {
			if packageVersion != installedPackages[packageName].Version {
				return nil, errors.Errorf("package %s-%s not installed", packageName, packageVersion)
			}
		} else {
			packageVersion = installedPackage.Version
		}

		fmt.Printf("remove %s-%s\n", packageName, packageVersion)
	}

	if no || !(yes || YesNo("continue?", "n")) {
		return installedPackages, nil
	}

	for _, p := range packages {
		packageName := GetPackageName(p)
		if err := RemovePackage(packageName, installedPackages, prefix); err != nil {
			return nil, errors.Wrapf(err, "failed to remove package %v", packageName)
		}
		delete(installedPackages, packageName)
	}
	return installedPackages, nil
}

func CheckPackages(
	packages []string,
	installedPackages map[string]InstalledPackage,
	prefix string,
) error {
	for _, p := range packages {
		packageName, packageVersion := SplitPackageNameVersion(p)
		if _, ok := installedPackages[packageName]; !ok || (packageVersion != "" && packageVersion != installedPackages[packageName].Version) {
			return errors.Errorf("%s not installed", packageName)
		}
	}

	errCount := 0
	for _, packageName := range packages {
		for fn := range installedPackages[packageName].Md5sums {
			filePath := filepath.Join(prefix, fn)
			if _, err := os.Stat(filePath); os.IsNotExist(err) {
				fmt.Printf("%s: does not exist: %s\n", packageName, filePath)
				errCount++
			} else if slices.Contains(installedPackages[packageName].Backup, fn) {
				continue
			} else if md5Sum, err := GetMD5Sum(filePath); md5Sum != installedPackages[packageName].Md5sums[fn] && err == nil {
				fmt.Printf("%s: checksum difference: %s\n", packageName, filePath)
				errCount++
			}
		}
	}
	if errCount > 0 {
		return errors.Errorf("error count: %v", errCount)
	}
	return nil
}

func ShowUntracked(
	paths []string,
	installedPackages map[string]InstalledPackage,
	prefix string,
) error {
	fnToPackageName := GenFnToPackageNameMapping(installedPackages, prefix)

	for _, path := range paths {
		_, files, err := GetRecursiveListing(prefix + "/" + path)
		if err != nil {
			return err
		}
		for _, fn := range files {
			fn = strings.TrimPrefix(fn, prefix+"/")
			if _, ok := fnToPackageName[prefix+"/"+path+"/"+fn]; !ok {
				fmt.Printf("unknown: %v\n", prefix+"/"+path+"/"+fn)
			}
		}
	}
	return nil
}

// TODO: add support for paths
func MergeConfig(
	packages []string,
	installedPackages map[string]InstalledPackage,
	prefix string,
) error {
	for _, p := range packages {
		packageName := GetPackageName(p)
		if _, ok := installedPackages[packageName]; !ok {
			return errors.Errorf("package %s not installed", packageName)
		}
	}
	for _, p := range packages {
		packageName := GetPackageName(p)
		if err := MergeConfigPackage(packageName, installedPackages, prefix); err != nil {
			return err
		}
	}
	return nil
}

func ListAvailable(
	packages []string,
	repos []string,
	offline bool,
) error {
	availablePackages := GetAvailablePackages(repos, offline)
	if len(packages) == 0 {
		ks := lo.Keys(availablePackages)
		slices.Sort(ks)
		for _, k := range ks {
			fmt.Println(k)
		}
	} else {
		for _, p := range packages {
			urls, ok := availablePackages[p]
			if !ok {
				return errors.Errorf("package %s not available", p)
			}
			for _, url := range urls {
				ver := GetPackageVersion(GetPackageFn(url))
				fmt.Printf("%s-%s\n", p, ver)
			}
		}
	}
	return nil
}

func IfInstalled(packages []string, installedPackages map[string]InstalledPackage) error {
	for _, p := range packages {
		packageName, packageVersion := SplitPackageNameVersion(p)
		installedPackage, ok := installedPackages[packageName]
		if !ok {
			return errors.Errorf("package %s not installed", packageName)
		}
		if packageVersion != "" && packageVersion != installedPackage.Version {
			return errors.Errorf("package %s-%s not installed", packageName, packageVersion)
		}
	}
	return nil
}