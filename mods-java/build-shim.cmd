@echo off
rem Builds shim\ (out-shim\), the gemjava test mod (out-mod\gemjava.jar) and runs the host on it.
rem Set SIDECAR_JDK to the JDK 25 folder.
setlocal
cd /d "%~dp0"
set "BIN=%SIDECAR_JDK%\bin\"
if not exist out-shim mkdir out-shim
if not exist out-mod mkdir out-mod
dir /s /b shim\src\*.java > out-shim\src.txt
"%BIN%javac.exe" --release 25 -d out-shim @out-shim\src.txt || exit /b 1
"%BIN%javac.exe" --release 25 -cp out-shim -d out-mod\classes poc-mods\gemjava\src\dev\horizon\gemjava\GemMod.java || exit /b 1
copy /y poc-mods\gemjava\fabric.mod.json out-mod\classes\ > nul
"%BIN%jar.exe" --create --file out-mod\gemjava.jar -C out-mod\classes . || exit /b 1
"%BIN%java.exe" -cp out-shim horizon.ModHost out-mod\gemjava.jar out-mod\gemjava-content.jar
exit /b %ERRORLEVEL%
