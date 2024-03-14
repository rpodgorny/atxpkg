package main

import (
	"crypto/tls"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"runtime"
	"strings"

	"github.com/docopt/docopt-go"
	"github.com/gookit/goutil/fsutil"
	"github.com/samber/lo"
)

const USAGE = `Asterix package manager.

Usage:
  atxpkg install [options] <package>...
  atxpkg update [options] [<package>...] [<old_package..new_package>...]
  atxpkg remove [options] <package>...
  atxpkg check [options] [<package>...]
  atxpkg merge_config [options] [<package>...]
  atxpkg list_available [options] [<package>...]
  atxpkg list_installed [options]
  atxpkg show_untracked [options] [<path>...]
  atxpkg clean_cache [options]

Options:
  -h,--help                     This screen.
  --force                       Force operation (overwrite files etc.)
  -w,--downloadonly             Only download packages, don't install/update anything.
  --prefix=<path>               Path prefix.
  -y,--yes                      Automatically answer yes to all questions.
  -n,--no                       Automatically answer no to all questions.
  --offline                     Don't connect to online repositories.
  --if-installed=<pkg,pkg,...>  Only perform install/update/remove if listed packages are installed.
  --unverified-ssl              Don't verify ssl certificate validity.
  --debug                       Enable debug mode.
  --version                     Print version.
`

func intMain() int {
	args := lo.Must(docopt.ParseArgs(USAGE, os.Args[1:], VERSION))

	//debug := lo.Must(args.Bool("--debug"))
	/*logLevel := "INFO"
	if debug {
		logLevel = "DEBUG"
	}*/
	//slog.SetFlags(slog.Ldate | slog.Ltime | slog.Lshortfile)
	//slog.SetPrefix("[atxpkg] ")

	slog.Info(fmt.Sprintf("starting atxpkg v%s", VERSION))

	//slog.Debug("args", "args", args)

	force := lo.Must(args.Bool("--force"))
	yes := lo.Must(args.Bool("--yes"))
	no := lo.Must(args.Bool("--no"))
	offline := lo.Must(args.Bool("--offline"))
	downloadOnly := lo.Must(args.Bool("--downloadonly"))
	unverifiedSSL := lo.Must(args.Bool("--unverified-ssl"))

	if unverifiedSSL {
		slog.Info("Overriding SSL context to be unverified")
		http.DefaultTransport.(*http.Transport).TLSClientConfig = &tls.Config{InsecureSkipVerify: true}
	}

	var dbFn, reposFn, cacheDir string
	var repos []string
	var prefix string

	if runtime.GOOS == "windows" {
		slog.Info("detected win32")
		dbFn = "c:/atxpkg/installed.json"
		reposFn = "c:/atxpkg/repos.txt"
		prefix = "c:"
		cacheDir = "c:/atxpkg/cache"
	} else {
		slog.Info("detected non-win32")
		dbFn = "/tmp/atxpkg/installed.json"
		reposFn = "/tmp/atxpkg/repos.txt"
		prefix = "/"
		cacheDir = "/tmp/atxpkg/cache"
	}

	if x, err := args.String("--prefix"); err == nil {
		prefix = x
	}
	//prefix = lo.Must(filepath.Abs(prefix))

	if !fsutil.IsDir(prefix) {
		slog.Error(fmt.Sprintf("prefix directory does not exist: %v", prefix))
		return 1
	}
	prefix = strings.TrimRight(prefix, "/")

	repos, err := GetRepos(reposFn)
	if err != nil {
		slog.Error(fmt.Sprintf("%+v", err))
		return 1
	}
	repos = append(repos, cacheDir)

	if !fsutil.FileExists(dbFn) {
		slog.Info(fmt.Sprintf("%s not found, creating empty one", dbFn))
		err := os.WriteFile(dbFn, []byte("{}"), 0o644)
		if err != nil {
			slog.Error("Error creating %s: %v", dbFn, err)
			return 1
		}
	}

	if !fsutil.IsDir(cacheDir) {
		slog.Info(fmt.Sprintf("%s not found, creating empty one", cacheDir))
		err := os.MkdirAll(cacheDir, os.ModePerm)
		if err != nil {
			slog.Error("Error creating %s: %v", cacheDir, err)
			return 1
		}
	}

	installedPackages, err := GetInstalledPackages(dbFn)
	if err != nil {
		slog.Error(fmt.Sprintf("%+v", err))
		return 1
	}

	if lo.Must(args.Bool("install")) {
		if ifInstalled, err := args.String("--if-installed"); err == nil {
			if err := IfInstalled(strings.Split(ifInstalled, ","), installedPackages); err != nil {
				slog.Error(fmt.Sprintf("%+v", err))
				return 1
			}
		}
		newInstalledPackages, err := InstallPackages(
			args["<package>"].([]string),
			installedPackages,
			prefix,
			repos,
			force,
			offline,
			yes,
			no,
			downloadOnly,
			cacheDir,
		)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
		err = SaveInstalledPackages(newInstalledPackages, dbFn)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
	} else if lo.Must(args.Bool("update")) {
		if ifInstalled, err := args.String("--if-installed"); err == nil {
			if err := IfInstalled(strings.Split(ifInstalled, ","), installedPackages); err != nil {
				slog.Error(fmt.Sprintf("%+v", err))
				return 1
			}
		}
		packages := args["<package>"].([]string)
		if len(packages) == 0 {
			packages = lo.Keys(installedPackages)
		}
		newInstalledPackages, err := UpdatePackages(
			packages,
			installedPackages,
			prefix,
			repos,
			force,
			offline,
			yes,
			no,
			downloadOnly,
			cacheDir,
		)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
		err = SaveInstalledPackages(newInstalledPackages, dbFn)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
	} else if lo.Must(args.Bool("merge_config")) {
		packages := args["<package>"].([]string)
		if len(packages) == 0 {
			packages = lo.Keys(installedPackages)
		}
		err := MergeConfig(
			packages,
			installedPackages,
			prefix,
		)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
	} else if lo.Must(args.Bool("remove")) {
		if ifInstalled, err := args.String("--if-installed"); err == nil {
			if err := IfInstalled(strings.Split(ifInstalled, ","), installedPackages); err != nil {
				slog.Error(fmt.Sprintf("%+v", err))
				return 1
			}
		}
		newInstalledPackages, err := RemovePackages(
			args["<package>"].([]string),
			installedPackages,
			prefix,
			yes,
			no,
		)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
		err = SaveInstalledPackages(newInstalledPackages, dbFn)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
	} else if lo.Must(args.Bool("list_available")) {
		err := ListAvailable(
			args["<package>"].([]string),
			repos,
			offline,
		)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
	} else if lo.Must(args.Bool("list_installed")) {
		for packageName, packageInfo := range installedPackages {
			fmt.Printf("%s-%s\n", packageName, packageInfo.Version)
		}
	} else if lo.Must(args.Bool("show_untracked")) {
		paths := args["<path>"].([]string)
		paths = lo.Map(paths, func(x string, _ int) string {
			return strings.Trim(x, "/")
		})
		if len(paths) == 0 {
			paths = []string{""}
		}
		err := ShowUntracked(
			paths,
			installedPackages,
			prefix,
		)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
	} else if lo.Must(args.Bool("clean_cache")) {
		err := CleanCache(cacheDir)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
	} else if lo.Must(args.Bool("check")) {
		packages := args["<package>"].([]string)
		if len(packages) == 0 {
			packages = lo.Keys(installedPackages)
		}
		err := CheckPackages(
			packages,
			installedPackages,
			prefix,
		)
		if err != nil {
			slog.Error(fmt.Sprintf("%+v", err))
			return 1
		}
	}
	slog.Debug("exit")
	return 0
}

func main() {
	os.Exit(intMain())
}
