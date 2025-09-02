@echo off
setlocal
rem Launch the Electron app without npm
set "APP_DIR=%~dp0"
pushd "%APP_DIR%"

if exist "node_modules\.bin\electron.cmd" (
  call "node_modules\.bin\electron.cmd" .
) else (
  echo Electron is not installed locally. Attempting to use npx...
  npx --yes electron@^30.0.0 .
)

popd
