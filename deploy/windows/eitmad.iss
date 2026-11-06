; Inno owns Windows package mechanics. Rust owns workshop setup and release policy.
#if VER < EncodeVer(7, 1, 1)
  #error Inno Setup 7.1.1 or later is required
#endif
#if InstallerRole == "server"
  #define InstallerName "EitmadServerSetup"
  #define DisplayName "الاعتماد - الخادم"
  #define OtherRole "client"
#else
  #if InstallerRole != "client"
    #error InstallerRole must be server or client
  #endif
  #define InstallerName "EitmadClientSetup"
  #define DisplayName "الاعتماد - جهاز الموظف"
  #define OtherRole "server"
#endif

[Setup]
AppId=Eitmad-{#InstallerRole}
AppName={#DisplayName}
AppVersion={#AppVersion}
AppPublisher=Eitmad
VersionInfoVersion={#WindowsVersion}
DefaultDirName={localappdata}\Programs\Eitmad\{#InstallerRole}
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
MinVersion=10.0.19041
OutputDir={#OutputDirectory}
OutputBaseFilename={#InstallerName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ShowLanguageDialog=no
UninstallDisplayName={#DisplayName}
UninstallDisplayIcon={app}\bundles\{#PayloadKey}\setup\EitmadSetup.exe
SignTool=eitmad
SignedUninstaller=yes
CloseApplications=no
RestartApplications=no
SetupLogging=no
UninstallLogging=no
SetupMutex=EitmadWindowsSetup

[Languages]
Name: "arabic"; MessagesFile: "compiler:Languages\Arabic.isl"

[Files]
Source: "{#PublisherCertificate}"; DestName: "publisher.cer"; Flags: dontcopy
; Keep complete immutable release trees. Recovery journals can refer to an earlier release.
Source: "{#PayloadDirectory}\*"; DestDir: "{app}\bundles\{#PayloadKey}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Registry]
Root: HKCU; Subkey: "Software\Eitmad\Installation\{#InstallerRole}"; ValueType: string; ValueName: "Directory"; ValueData: "{app}"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Eitmad\Installation"; Flags: uninsdeletekeyifempty
Root: HKCU; Subkey: "Software\Eitmad"; Flags: uninsdeletekeyifempty

[Icons]
Name: "{userprograms}\{#DisplayName}\استكمال الإعداد"; Filename: "{app}\bundles\{#PayloadKey}\setup\EitmadSetup.exe"; Parameters: "--role {#InstallerRole}"
Name: "{userprograms}\{#DisplayName}\إزالة الاعتماد"; Filename: "{uninstallexe}"

[UninstallDelete]
Type: dirifempty; Name: "{localappdata}\Programs\Eitmad"

[Messages]
FinishedHeadingLabel=اكتمل تجهيز ملفات الإعداد
FinishedLabel=ستفتح خطوات إعداد الاعتماد الآن. لا يكتمل تثبيت التطبيق حتى يؤكد إعداد الاعتماد نجاحه.
ConfirmUninstall=هل تريد إزالة %1 وخدماته وإعدادات التثبيت؟%n%nتبقى بيانات الورشة والسجلات المحلية محفوظة خارج مسارات التثبيت النشطة. التثبيت التالي يبدأ بإعداد جديد، ولا يستعيد هذه البيانات تلقائيًا.

[Code]
type
  TPathLocks = array of NativeInt;
  TToolLocks = array of TFileStream;

function OpenDirectory(Path: String; Access, Share: DWORD; Security: NativeUInt;
  Creation, Flags: DWORD; Template: NativeInt): NativeInt;
  external 'CreateFileW@kernel32.dll stdcall';
function ClosePath(Handle: NativeInt): BOOL;
  external 'CloseHandle@kernel32.dll stdcall';

procedure RetainParents(Path: String; var Locks: TPathLocks);
var Parent: String; Handle: NativeInt; Count: Integer;
begin
  Parent := ExtractFileDir(Path);
  if Parent <> Path then RetainParents(Parent, Locks);
  { Lock ancestors first. Deny writes and renames while an elevated child reads them. }
  Handle := OpenDirectory(Path, 0, 1, 0, 3, $02000000, 0);
  if Handle = -1 then RaiseException('تعذر حماية مسار ملفات الإعداد. أغلق البرامج التي تستخدمه وأعد المحاولة.');
  Count := GetArrayLength(Locks);
  SetArrayLength(Locks, Count + 1);
  Locks[Count] := Handle;
end;

procedure ReleaseParents(var Locks: TPathLocks);
var Index: Integer;
begin
  for Index := GetArrayLength(Locks) - 1 downto 0 do ClosePath(Locks[Index]);
  SetArrayLength(Locks, 0);
end;

var
  TrustPage: TInputOptionWizardPage;
  SetupExitCode: Integer;
  PublisherTrusted: Boolean;

function PSString(Value: String): String;
begin
  StringChangeEx(Value, '''', '''''', True);
  Result := '''' + Value + '''';
end;

function PowerShell(Command: String): Boolean;
var Code: Integer;
begin
  Result := Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    '-NoProfile -NonInteractive -Command "' + Command + '"', '', SW_HIDE, ewWaitUntilTerminated, Code);
  Result := Result and (Code = 0);
end;

function PublisherInStore(StoreName: String): Boolean;
begin
  Result := PowerShell('$ErrorActionPreference=''Stop''; ' +
    '$wanted=''{#PublisherCertificateSha256}''; ' +
    '$found=@(Get-ChildItem ' + PSString('Cert:\LocalMachine\' + StoreName) + ' | Where-Object ' +
    '{[BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash($_.RawData)).Replace(''-'','''') -eq $wanted}); ' +
    'if($found.Count -eq 0){exit 1}');
end;

function TrustedPublisher: Boolean;
begin
  Result := PublisherInStore('Root') and PublisherInStore('TrustedPublisher');
end;

function ValidSignature(Path: String): Boolean;
begin
  Result := PowerShell('$ErrorActionPreference=''Stop''; $s=Get-AuthenticodeSignature -LiteralPath ' + PSString(Path) + '; ' +
    'if($s.Status -ne ''Valid''){exit 1}; ' +
    'if([BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash($s.SignerCertificate.RawData)).Replace(''-'','''') ' +
    '-ne ''{#PublisherCertificateSha256}''){exit 1}');
end;

function InitializeSetup: Boolean;
begin
  Result := not IsAdmin;
  if not Result then
    MsgBox('افتح ملف الإعداد بحساب المستخدم العادي. لا تستخدم تشغيل كمسؤول.', mbError, MB_OK);
  if Result and RegKeyExists(HKCU, 'Software\Eitmad\Installation\{#OtherRole}') then begin
    MsgBox('يوجد إعداد الاعتماد للدور الآخر لهذا المستخدم. أزله أولًا قبل تغيير دور الجهاز.', mbError, MB_OK);
    Result := False;
  end;
  if Result and WizardSilent then begin
    MsgBox('يتطلب إعداد الورشة تأكيدًا وخطوات تفاعلية. افتح ملف الإعداد دون الوضع الصامت.', mbError, MB_OK);
    Result := False;
  end;
end;

procedure InitializeWizard;
begin
  PublisherTrusted := TrustedPublisher;
  TrustPage := CreateInputOptionPage(wpWelcome, 'اعتماد ناشر الاعتماد',
    'قارن بصمة الشهادة مع المصدر الذي سلّمك ملف الإعداد.',
    'يستخدم الاعتماد شهادة ناشر ذاتية التوقيع. بموافقة مسؤول Windows، تُضاف الشهادة العامة إلى مخزني الجذور والناشرين الموثوقين لهذا الجهاز. قد يظهر طلبان.' + #13#10#13#10 +
    '' + #$202A + 'SHA-256: {#PublisherCertificateSha256}' + #$202C, False, False);
  TrustPage.Add('قارنت البصمة وأوافق على اعتماد هذا الناشر لهذا الجهاز');
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := (PageID = TrustPage.ID) and PublisherTrusted;
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := (CurPageID <> TrustPage.ID) or TrustPage.Values[0];
  if not Result then
    MsgBox('قارن بصمة الشهادة وأكد الموافقة قبل المتابعة.', mbInformation, MB_OK);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var Code: Integer; CertificatePath: String; CertificateLock: TFileStream;
  Parents: TPathLocks;
begin
  Result := '';
  if not TrustedPublisher then begin
    if not TrustPage.Values[0] then begin Result := 'لم يتم تأكيد اعتماد الناشر.'; Exit; end;
    ExtractTemporaryFile('publisher.cer');
    CertificatePath := ExpandConstant('{tmp}\publisher.cer');
    try
      RetainParents(ExtractFileDir(CertificatePath), Parents);
      CertificateLock := TFileStream.Create(CertificatePath, fmOpenRead or fmShareDenyWrite);
      try
    if CompareText(GetSHA256OfFile(CertificatePath), '{#PublisherCertificateSha256}') <> 0 then begin
      Result := 'بصمة شهادة الناشر غير مطابقة.'; Exit;
    end;
    if not PublisherInStore('Root') and (not ShellExec('runas', ExpandConstant('{sys}\certutil.exe'), '-addstore Root "' + CertificatePath + '"', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0)) then begin
      Result := 'تعذر اعتماد شهادة الناشر. لم يكتمل التثبيت.'; Exit;
    end;
    if not PublisherInStore('TrustedPublisher') and (not ShellExec('runas', ExpandConstant('{sys}\certutil.exe'), '-addstore TrustedPublisher "' + CertificatePath + '"', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0)) then begin
      Result := 'تعذر اعتماد الناشر الموثوق. لم يكتمل التثبيت.'; Exit;
    end;
      finally
        CertificateLock.Free;
      end;
    finally
      ReleaseParents(Parents);
    end;
  end;
  if not TrustedPublisher or not ValidSignature(ExpandConstant('{srcexe}')) then
    Result := 'تعذر التحقق من توقيع ملف الإعداد. استخدم النسخة المعتمدة.';
end;

procedure CurStepChanged(CurStep: TSetupStep);
var Code: Integer; Setup: String;
begin
  if CurStep = ssPostInstall then begin
    SetupExitCode := 1;
    Setup := ExpandConstant('{app}\bundles\{#PayloadKey}\setup\EitmadSetup.exe');
    if not ValidSignature(Setup) then RaiseException('تعذر التحقق من توقيع برنامج إعداد الاعتماد.');
    WizardForm.Hide;
    try
      if not Exec(Setup, '--role {#InstallerRole}', '', SW_SHOWNORMAL, ewWaitUntilTerminated, Code) then
        Code := 1;
      SetupExitCode := Code;
    finally
      WizardForm.Show;
    end;
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpFinished then begin
    if SetupExitCode <> 0 then begin
      WizardForm.FinishedHeadingLabel.Caption := 'لم يكتمل إعداد الاعتماد';
      WizardForm.FinishedLabel.Caption := 'ملفات الاستكمال والإزالة محفوظة. افتح استكمال الإعداد من قائمة ابدأ، أو أزل الاعتماد قبل إعداد جديد.';
    end else begin
      WizardForm.FinishedHeadingLabel.Caption := 'اكتمل تثبيت الاعتماد';
      WizardForm.FinishedLabel.Caption := 'أكد إعداد الاعتماد اكتمال التثبيت. يمكنك فتح التطبيق من قائمة ابدأ.';
    end;
  end;
end;

function GetCustomSetupExitCode: Integer;
begin
  Result := SetupExitCode;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var Script, Parameters: String; Code: Integer; Started: Boolean;
  Parents: TPathLocks; Tools: TToolLocks; Names: TArrayOfString; Index: Integer;
begin
  if CurUninstallStep = usUninstall then begin
    if IsAdmin then RaiseException('شغّل الإزالة بحساب المستخدم العادي الذي ثبّت الاعتماد.');
    Script := ExpandConstant('{app}\bundles\{#PayloadKey}\server\application\deploy\windows\uninstall.ps1');
    try
      RetainParents(ExtractFileDir(Script), Parents);
      Names := ['uninstall.ps1', 'uninstallation.psm1', 'host.psm1', 'package.psm1', 'verify-offline.ps1'];
      SetArrayLength(Tools, GetArrayLength(Names));
      for Index := 0 to GetArrayLength(Names) - 1 do
        Tools[Index] := TFileStream.Create(AddBackslash(ExtractFileDir(Script)) + Names[Index], fmOpenRead or fmShareDenyWrite);
    if not ValidSignature(Script) then RaiseException('تعذر التحقق من برنامج الإزالة. أصلح ملفات الإعداد بالنسخة المعتمدة ثم أعد المحاولة.');
    Parameters := '-NoProfile -NonInteractive -ExecutionPolicy AllSigned -File "' + Script + '" -Role {#InstallerRole} -PublisherCertificateSha256 {#PublisherCertificateSha256} -Phase ';
    Started := Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
      Parameters + 'CheckUser', '', SW_HIDE, ewWaitUntilTerminated, Code);
    if not Started or (Code <> 0) then RaiseException('أغلق الاعتماد وبرنامج الإعداد قبل الإزالة. لم تُحذف ملفات الإزالة.');
#if InstallerRole == "server"
    Started := ShellExec('runas', ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
      Parameters + 'Machine', '', SW_HIDE, ewWaitUntilTerminated, Code);
    if not Started or (Code <> 0) then RaiseException('لم تكتمل إزالة خدمات الاعتماد أو حفظ بيانات الورشة. احتفظ بملفات الإعداد وأعد المحاولة بعد فحص الخدمات وموافقة مسؤول Windows.');
#endif
    Started := Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
      Parameters + 'User', '', SW_HIDE, ewWaitUntilTerminated, Code);
    if not Started or (Code <> 0) then RaiseException('لم تكتمل إزالة تطبيق هذا المستخدم. أغلق الاعتماد ثم أعد المحاولة. بقيت ملفات الإزالة والبيانات محفوظة.');
    finally
      for Index := GetArrayLength(Tools) - 1 downto 0 do
        if Tools[Index] <> nil then Tools[Index].Free;
      ReleaseParents(Parents);
    end;
  end;
end;
