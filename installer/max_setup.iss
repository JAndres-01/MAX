; ========================================================
; Script de Instalación de MAX para Windows (x64)
; Motor Inno Setup 6+
; ========================================================

#define MyAppName "MAX"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "MAX Development Team"
#define MyAppURL "https://github.com/hyprand/MAX"
#define MyAppExeName "max_drop.exe"

[Setup]
AppId={{D37E8842-124E-4BC9-9A07-7FE3E59B36DA}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DisableProgramGroupPage=yes
; Permite instalación para el usuario actual sin requerir elevación obligatoria de Administrador
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=output
OutputBaseFilename=MAX-Setup-x64
SetupIconFile=..\ui\windows\runner\resources\app_icon.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
WizardResizable=no
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "startup"; Description: "Iniciar MAX automáticamente al encender Windows"; GroupDescription: "Opciones de inicio:"

[Files]
; Empaqueta todo el paquete generado por Flutter y el motor max_core.dll
Source: "..\ui\build\windows\x64\runner\Release\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppExeName}"
Name: "{autoprograms}\{#MyAppName} Settings"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--settings"; IconFilename: "{app}\{#MyAppExeName}"; Comment: "Configuración de MAX"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppExeName}"; Tasks: desktopicon
Name: "{autodesktop}\{#MyAppName} Settings"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--settings"; IconFilename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; Comment: "Configuración de MAX"

[Registry]
; Registro en Windows Run para autoinicio opcional
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#MyAppName}"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: startup

[Run]
; Regla de firewall para permitir transferencias P2P de LocalSend en redes locales (puertos 53317 y 53320)
Filename: "netsh"; Parameters: "advfirewall firewall add rule name=""MAX LocalSend P2P"" dir=in action=allow program=""{app}\{#MyAppExeName}"" enable=yes profile=any"; Flags: runhidden; StatusMsg: "Configurando acceso seguro en red local..."
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "netsh"; Parameters: "advfirewall firewall delete rule name=""MAX LocalSend P2P"""; Flags: runhidden
