!define ARACHNID_INSTALLER_DIR "${__FILEDIR__}"

!macro NSIS_HOOK_POSTINSTALL
  InitPluginsDir
  File "/oname=$PLUGINSDIR\tasm-prerequisites.ps1" "${ARACHNID_INSTALLER_DIR}\tasm-prerequisites.ps1"

  DetailPrint "Checking The Amazing Spider-Man prerequisites..."
  System::Call 'kernel32::SetEnvironmentVariableW(w "ARACHNID_PREREQUISITE_SCRIPT", w "$PLUGINSDIR\tasm-prerequisites.ps1") i'
  StrCpy $R8 "$SYSDIR"
  IfFileExists "$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe" 0 +2
    StrCpy $R8 "$WINDIR\Sysnative"
  nsExec::ExecToLog '"$R8\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -Command "& ([scriptblock]::Create([IO.File]::ReadAllText($$env:ARACHNID_PREREQUISITE_SCRIPT)))"'
  Pop $R9
  System::Call 'kernel32::SetEnvironmentVariableW(w "ARACHNID_PREREQUISITE_SCRIPT", p 0) i'

  StrCmp $R9 "0" prerequisites_done
  StrCmp $R9 "3010" prerequisites_reboot
  StrCmp $R9 "3011" prerequisites_reboot_with_errors
  Goto prerequisites_failed
  prerequisites_reboot_with_errors:
    SetRebootFlag true
    Goto prerequisites_failed
  prerequisites_reboot:
    SetRebootFlag true
    Goto prerequisites_done
  prerequisites_failed:
    MessageBox MB_OK|MB_ICONEXCLAMATION "Arachnid was installed, but some game prerequisites could not be installed. See the installation details, then run this installer again to retry." /SD IDOK
  prerequisites_done:
  DetailPrint "Prerequisite check finished (exit code $R9)."
!macroend
