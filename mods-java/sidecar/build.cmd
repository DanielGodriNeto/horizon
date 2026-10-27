@echo off
rem Compila Sidecar e SelfTest em out\. Para usar outro JDK 25, defina SIDECAR_JDK com a pasta do JDK.
setlocal
cd /d "%~dp0"
set "JAVAC=javac"
if defined SIDECAR_JDK set "JAVAC=%SIDECAR_JDK%\bin\javac.exe"
if not exist out mkdir out
"%JAVAC%" --release 25 -d out src\Sidecar.java src\SelfTest.java
exit /b %ERRORLEVEL%
