@echo off
rem Plays SanctuaryRPG inside the console this is run from, instead of in its own window.
rem Needs sanctuary-pad to be installed; see its README.
setlocal
set SANCTUARY_PAD_TERMINAL=1
start "" /d "%~dp0." /wait "%~dp0SanctuaryRPG.exe"
