@echo off
rem Runs the web family gate and writes its console output next to verify.json.
cd /d "%~dp0.."
node scripts\verify.ts %* > scripts\verify.log 2>&1
echo exit=%ERRORLEVEL%>> scripts\verify.log
