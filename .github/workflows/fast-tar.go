// Wrapper that delegates to Windows native bsdtar (C:\Windows\System32\tar.exe)
// instead of the slow MSYS2 tar from Git for Windows.
//
// The only change: strips --force-local, which GNU tar needs but bsdtar doesn't
// recognize. All other flags (-xf, -P, -C, --use-compress-program) are compatible.
package main

import (
	"os"
	"os/exec"
)

func main() {
	var args []string
	for _, a := range os.Args[1:] {
		if a != "--force-local" {
			args = append(args, a)
		}
	}
	cmd := exec.Command(`C:\Windows\System32\tar.exe`, args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Stdin = os.Stdin
	if err := cmd.Run(); err != nil {
		if e, ok := err.(*exec.ExitError); ok {
			os.Exit(e.ExitCode())
		}
		os.Exit(1)
	}
}
