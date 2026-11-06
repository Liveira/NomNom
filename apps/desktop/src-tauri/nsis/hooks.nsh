; NOMNOM installer hooks. The installer runs per user, so nothing here needs administrator rights.
;
; After install: set up whatever the user has already chosen. On a first install that is nothing,
; because NomNom asks on its first run (right-click menu, Firefox, start with Windows).
; Before uninstall: remove everything NomNom added outside its own folder: the right-click menu
; (Windows 11 package and classic entries), the Firefox connection and the start-with-Windows entry.

!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec '"$INSTDIR\nomnom.exe" --install-integrations'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec '"$INSTDIR\nomnom.exe" --uninstall-integrations'
!macroend
