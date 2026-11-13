param(
    [Parameter(Mandatory)][string]$LoaderSource,
    [Parameter(Mandatory)][string]$RetexSource,
    [Parameter(Mandatory)][string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$loaderRoot = (Resolve-Path -LiteralPath $LoaderSource).Path
$retexRoot = (Resolve-Path -LiteralPath $RetexSource).Path
if ((git -C $loaderRoot rev-parse HEAD) -ne '59923458c3683f8034ce87e287d51f76a959bf41') { throw 'Unexpected ASI Loader revision' }
if ((git -C $retexRoot rev-parse HEAD) -ne '5b2d2cddb44280b526a5c9eb3bb492f281ad06e7') { throw 'Unexpected Retex revision' }
if (git -C $loaderRoot status --porcelain --untracked-files=no) { throw 'Modified ASI Loader sources' }
if (git -C $retexRoot status --porcelain --untracked-files=no) { throw 'Modified Retex sources' }
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null

# Restrict upstream plugin discovery to the single managed hook. No modloader,
# recursive ASI discovery, window-mode payload, or other embedded DLL is built.
$source = Get-Content -LiteralPath "$loaderRoot/source/dllmain.cpp" -Raw
$start = $source.IndexOf('void LoadPlugins()')
$end = $source.IndexOf('static LONG LoadedPluginsYet', $start)
if ($start -lt 0 -or $end -le $start) { throw 'Unexpected plugin loader structure' }
$restricted = @'
void LoadPlugins()
{
    const auto path = std::filesystem::path(GetModuleFileNameW(hm)).parent_path() / L"arachnid_skin.asi";
    auto module = LoadLibraryExW(path.c_str(), nullptr, LOAD_WITH_ALTERED_SEARCH_PATH);
    if (module != nullptr)
    {
        auto initialize = reinterpret_cast<void(*)()>(GetProcAddress(module, "InitializeASI"));
        if (initialize != nullptr) initialize();
    }
}

'@
$source = $source.Substring(0, $start) + $restricted + $source.Substring($end)
$source = [regex]::Replace($source, '(?m)^    iniPaths\.emplace_back\([^\r\n]+\);\r?\n', '')
$source = $source.Replace('TEXT("disablecrashdumps"), FALSE, iniPaths', 'TEXT("disablecrashdumps"), TRUE, iniPaths')
$source = $source.Replace('TEXT("overloadfromfolder"), TEXT("update"), iniPaths', 'TEXT("overloadfromfolder"), TEXT(""), iniPaths')
$source = $source.Replace('    Direct3D8DisableMaximizedWindowedModeShim();', '')
$originalLoader = [regex]::Match($source, '(?s)void LoadOriginalLibrary\(\)\r?\n\{.*?\r?\n\}')
if (-not $originalLoader.Success) { throw 'Unexpected original library loader structure' }
$systemOnly = @'
void LoadOriginalLibrary()
{
    if (_InterlockedCompareExchange(&OriginalLibraryLoaded, 1, 0) != 0) return;
    wchar_t directory[MAX_PATH];
    const auto length = GetSystemDirectoryW(directory, MAX_PATH);
    if (length == 0 || length >= MAX_PATH) return;
    const auto path = std::wstring(directory) + L"\\dinput8.dll";
    dinput8.LoadOriginalLibrary(LoadLibraryExW(path.c_str(), nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32));
}
'@
$source = $source.Remove($originalLoader.Index, $originalLoader.Length).Insert($originalLoader.Index, $systemOnly)
[IO.File]::WriteAllText("$outputRoot/dllmain.cpp", $source)
@'
LIBRARY dinput8
EXPORTS
DirectInput8Create = _DirectInput8Create
DllCanUnloadNow = _DllCanUnloadNow PRIVATE
DllGetClassObject = _DllGetClassObject PRIVATE
DllRegisterServer = _DllRegisterServer PRIVATE
DllUnregisterServer = _DllUnregisterServer PRIVATE
'@ | Set-Content -LiteralPath "$outputRoot/dinput8.def"

$files = @("$outputRoot/dllmain.cpp", "$loaderRoot/external/MemoryModule/MemoryModule.c", "$loaderRoot/external/injector/utility/FunctionHookMinHook.cpp")
$files += Get-ChildItem -LiteralPath "$loaderRoot/external/d3d8to9/source" -Filter '*.cpp' | Select-Object -ExpandProperty FullName
$files += Get-ChildItem -LiteralPath "$loaderRoot/external/injector/minhook/src" -Filter '*.c' -Recurse | Select-Object -ExpandProperty FullName
$items = ($files | ForEach-Object { '<ClCompile Include="' + [Security.SecurityElement]::Escape($_) + '" />' }) -join "`n"
$includes = "$loaderRoot/source;$loaderRoot/external;$loaderRoot/external/injector/minhook/include;$loaderRoot/external/injector/utility;$loaderRoot/external/minidx9/Include"
$project = @"
<Project DefaultTargets="Build" xmlns="http://schemas.microsoft.com/developer/msbuild/2003">
  <ItemGroup Label="ProjectConfigurations"><ProjectConfiguration Include="Release|Win32"><Configuration>Release</Configuration><Platform>Win32</Platform></ProjectConfiguration></ItemGroup>
  <PropertyGroup Label="Globals"><WindowsTargetPlatformVersion>10.0</WindowsTargetPlatformVersion></PropertyGroup>
  <Import Project="`$(VCTargetsPath)\Microsoft.Cpp.Default.props" />
  <PropertyGroup Label="Configuration"><ConfigurationType>DynamicLibrary</ConfigurationType><PlatformToolset>v143</PlatformToolset><CharacterSet>Unicode</CharacterSet></PropertyGroup>
  <Import Project="`$(VCTargetsPath)\Microsoft.Cpp.props" />
  <PropertyGroup><OutDir>$outputRoot\bin\</OutDir><IntDir>$outputRoot\obj\</IntDir><TargetName>dinput8</TargetName><LinkIncremental>false</LinkIncremental></PropertyGroup>
  <ItemDefinitionGroup>
    <ClCompile><AdditionalIncludeDirectories>$includes</AdditionalIncludeDirectories><PreprocessorDefinitions>NDEBUG;D3D8TO9NOLOG;UNICODE;_UNICODE;%(PreprocessorDefinitions)</PreprocessorDefinitions><LanguageStandard>stdcpplatest</LanguageStandard><ExceptionHandling>Async</ExceptionHandling><RuntimeLibrary>MultiThreaded</RuntimeLibrary><Optimization>MaxSpeed</Optimization><MultiProcessorCompilation>true</MultiProcessorCompilation></ClCompile>
    <Link><ModuleDefinitionFile>$outputRoot/dinput8.def</ModuleDefinitionFile><AdditionalLibraryDirectories>$loaderRoot/external/minidx9/Lib/x86</AdditionalLibraryDirectories><AdditionalDependencies>version.lib;shlwapi.lib;%(AdditionalDependencies)</AdditionalDependencies><AdditionalOptions>/Brepro %(AdditionalOptions)</AdditionalOptions></Link>
  </ItemDefinitionGroup>
  <ItemGroup>$items</ItemGroup>
  <Import Project="`$(VCTargetsPath)\Microsoft.Cpp.targets" />
</Project>
"@
[IO.File]::WriteAllText("$outputRoot/skin-loader.vcxproj", $project)
$msbuild = 'C:/Program Files/Microsoft Visual Studio/2022/Community/MSBuild/Current/Bin/MSBuild.exe'
& $msbuild "$outputRoot/skin-loader.vcxproj" /p:Configuration=Release /p:Platform=Win32 /nologo /verbosity:minimal
if ($LASTEXITCODE -ne 0) { throw 'ASI Loader build failed' }
Push-Location $retexRoot
try {
    cargo build --release --locked -p retex-hook --target i686-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Retex hook build failed' }
} finally { Pop-Location }
Copy-Item -LiteralPath "$retexRoot/target/i686-pc-windows-msvc/release/retex_hook.dll" -Destination "$outputRoot/bin/arachnid_skin.asi"
Get-FileHash -LiteralPath "$outputRoot/bin/dinput8.dll", "$outputRoot/bin/arachnid_skin.asi" -Algorithm SHA256
