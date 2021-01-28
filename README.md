# Custom Build Tool
This is my own custom build tool made in rust-lang. 

The premise is that it is a single executable that parses a build.cfg. The build cfg can have variables, tasks, and an execution order. Below is an example.

# Example Build.cfg
```ini
$sources = test/main.cpp
$output = test/test.exe

[build]
command = g++ $sources -o $output

[run]
command = ./test/test.exe

[execute]
build
run
```

# Example Output
```
info: reading build.cfg...
info: found 2 var(s) and 2 task(s)
task(build): finished
task(run): finished

Hello, World! 
```
