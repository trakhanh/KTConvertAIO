; Extra steps for the KT Convert installer (see bundle.windows.nsis.installerHooks).

!macro NSIS_HOOK_POSTUNINSTALL
  ; Remove the "start with Windows" entry the app writes (winutil::set_autostart).
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "KT Convert"
!macroend
