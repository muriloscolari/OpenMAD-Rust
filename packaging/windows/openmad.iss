; Inno Setup installer for OpenMAD-Rust (Windows x64)

#define VerFile   AddBackslash(SourcePath) + "..\..\version.ini"
#define AppName    ReadIni(VerFile, "app", "name", "OpenMAD")
#define AppVersion ReadIni(VerFile, "app", "version", "0.1.0")
#define Publisher  ReadIni(VerFile, "app", "publisher", "Murilo Scolari")
#define AppUrl     ReadIni(VerFile, "app", "url", "https://github.com/muriloscolari/OpenMAD-Rust")
#define AppExe     "OpenMAD.exe"

[Setup]
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#Publisher}
AppPublisherURL={#AppUrl}
AppSupportURL={#AppUrl}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
OutputDir=Output
OutputBaseFilename={#AppName}-Setup-{#AppVersion}
SetupIconFile=..\..\assets\icon.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#AppExe}
CloseApplications=force
RestartApplications=no

[Files]
Source: "..\..\target\release\openmad.exe"; DestDir: "{app}"; DestName: "{#AppExe}"; Flags: ignoreversion
Source: "..\..\assets\icon.ico"; DestDir: "{app}\assets"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{group}\{#AppName}";       Filename: "{app}\{#AppExe}"; IconFilename: "{app}\assets\icon.ico"; Tasks: startmenuicon
Name: "{userdesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; IconFilename: "{app}\assets\icon.ico"; Tasks: desktopicon

[Tasks]
Name: "startup";       Description: "Start automatically minimized to system tray on sign-in"
Name: "desktopicon";   Description: "Create a desktop shortcut"
Name: "startmenuicon"; Description: "Create a Start-menu shortcut"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; \
    ValueName: "{#AppName}"; ValueData: """{app}\{#AppExe}"" --minimized"; \
    Tasks: startup; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
