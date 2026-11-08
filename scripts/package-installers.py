#!/usr/bin/env python3
"""Build and verify engineering installers on native GitHub runners; never publish."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import platform
import plistlib
import re
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
APP_ID = "app.writercraft.desktop"
NSIS_VERSION = "3.12"
TARGETS = {
    "macos-arm64": ("Darwin", "macOS", "aarch64-apple-darwin", ".dmg"),
    "windows-x64": ("Windows", "Windows", "x86_64-pc-windows-msvc", "-setup.exe"),
    "linux-amd64": ("Linux", "Linux", "x86_64-unknown-linux-gnu", ".deb"),
}
# winit/wgpu load these at runtime, so ELF NEEDED alone cannot discover them.
LINUX_RUNTIME = (
    "libx11-6", "libx11-xcb1", "libxcb1", "libxcb-render0", "libxcb-shape0",
    "libxcb-xfixes0", "libxcursor1", "libxi6", "libxrandr2", "libxinerama1",
    "libxkbcommon0", "libxkbcommon-x11-0", "libwayland-client0", "libwayland-cursor0",
    "libwayland-egl1", "libegl1", "libgl1", "libvulkan1", "xdg-desktop-portal",
)


def run(arguments, **kwargs):
    return subprocess.run([str(item) for item in arguments], check=True, **kwargs)


def capture(arguments, **kwargs):
    return run(arguments, text=True, stdout=subprocess.PIPE, **kwargs).stdout.strip()


def load_script(name):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), ROOT / "scripts" / (name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def require_hosted(name, environment=None, host=None):
    environment = os.environ if environment is None else environment
    host = platform.system() if host is None else host
    if environment.get("GITHUB_ACTIONS") != "true":
        raise ValueError("Installer builds and install/remove checks run only on GitHub Actions")
    expected_host, expected_runner, _, _ = TARGETS[name]
    if host != expected_host or environment.get("RUNNER_OS") != expected_runner:
        raise ValueError("Installer requires its native " + expected_runner + " runner")


def windows_version(version):
    """Windows PE resources need four integers; exact SemVer remains in package metadata."""
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:-(alpha|beta|rc)\.(\d+))?", version)
    if not match:
        raise ValueError("Installer version must be X.Y.Z or X.Y.Z-(alpha|beta|rc).N")
    major, minor, patch = (int(value) for value in match.groups()[:3])
    phase, sequence = match.groups()[3:]
    sequence = int(sequence) if sequence else 0
    if phase and not 1 <= sequence <= 299:
        raise ValueError("Windows prerelease sequence must be between 1 and 299")
    build = {"alpha": 0, "beta": 300, "rc": 600}[phase] + sequence if phase else 999
    if any(value > 65535 for value in [major, minor, patch, build]):
        raise ValueError("Version exceeds Windows PE numeric limits")
    return f"{major}.{minor}.{patch}.{build}"


def debian_version(version):
    windows_version(version)  # Use one supported release grammar on every platform.
    return version.replace("-", "~", 1)


def icon_from_icns(source, destination):
    data = source.read_bytes()
    if data[:4] != b"icns" or len(data) < 8 or struct.unpack_from(">I", data, 4)[0] != len(data):
        raise ValueError("Invalid ICNS container")
    position = 8
    while position + 8 <= len(data):
        kind, length = struct.unpack_from(">4sI", data, position)
        if length < 8 or position + length > len(data):
            raise ValueError("Invalid ICNS icon length")
        png = data[position + 8:position + length]
        if kind == b"ic08" and png[:8] == b"\x89PNG\r\n\x1a\n" and len(png) >= 24 and struct.unpack_from(">II", png, 16) == (256, 256):
            # Windows Vista+ accepts a PNG image in an ICO; zero width/height denotes 256.
            destination.write_bytes(struct.pack("<HHHBBBBHHII", 0, 1, 1, 0, 0, 0, 0, 1, 32, len(png), 22) + png)
            return
        position += length
    raise ValueError("ICNS needs its original 256px PNG (ic08) for the Windows icon")


def stage_payload(root, binary, destination, metadata, notices):
    destination.mkdir(parents=True)
    shutil.copy2(binary, destination / ("WriterCraft.exe" if metadata["platform"] == "windows-x64" else "writercraft"))
    for name, source in [("LICENSE", root / "LICENSE"), ("THIRD_PARTY_NOTICES.txt", notices)]:
        if source.is_symlink() or not source.is_file() or not source.stat().st_size:
            raise ValueError("Missing regular nonempty notice: " + name)
        shutil.copy2(source, destination / name)
    for source in sorted((root / "assets").rglob("*")):
        if source.is_file() and ("LICENSE" in source.name or source.name == "NOTICE.md"):
            if source.is_symlink():
                raise ValueError("Notice symlinks cannot enter the package")
            target = destination / "licenses" / source.relative_to(root / "assets")
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
    if not any((destination / "licenses/fonts").glob("*LICENSE*")):
        raise ValueError("Bundled font license notices are missing")
    write_json(destination / "package-metadata.json", metadata)


def nsis_literal(value):
    value = str(value)
    if "\n" in value or "\r" in value:
        raise ValueError("NSIS strings cannot contain newlines")
    return '"' + str(value).replace("$", "$$").replace('"', '$\\"') + '"'


def nsis_source(payload, metadata, output, package):
    header = r'''Unicode true
!include LogicLib.nsh
!include x64.nsh
!include WinVer.nsh
!include Win\COM.nsh
!include Win\Propkey.nsh
RequestExecutionLevel user
SetCompressor zlib
SetOverwrite on
Name "WriterCraft"
InstallDir "$LOCALAPPDATA\Programs\WriterCraft"
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles
Function .onInit
  SetShellVarContext current
  SetRegView 64
  ${IfNot} ${RunningX64}
    Abort "WriterCraft requires 64-bit Windows."
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    Abort "WriterCraft requires Windows 10 or newer."
  ${EndIf}
FunctionEnd
Function un.onInit
  SetShellVarContext current
  SetRegView 64
FunctionEnd

; Built-in NSIS COM helpers set the same identity as the native application.
Function CreateStartMenu
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
  ${If} $0 P= 0
    Goto shortcut_failed
  ${EndIf}
  ${IShellLink::SetPath} $0 '("$INSTDIR\WriterCraft.exe").r9'
  ${If} $9 <> 0
    Goto shortcut_failed
  ${EndIf}
  ${IShellLink::SetWorkingDirectory} $0 '("$INSTDIR").r9'
  ${If} $9 <> 0
    Goto shortcut_failed
  ${EndIf}
  ${IShellLink::SetIconLocation} $0 '("$INSTDIR\WriterCraft.ico",0).r9'
  ${If} $9 <> 0
    Goto shortcut_failed
  ${EndIf}
  ${IUnknown::QueryInterface} $0 '("${IID_IPropertyStore}",.r1)'
  ${If} $1 P= 0
    Goto shortcut_failed
  ${EndIf}
  System::Call 'shlwapi::SHStrDupW(w "app.writercraft.desktop", *p.r4)i.r9'
  ${If} $9 <> 0
    Goto shortcut_failed
  ${EndIf}
  System::Call '*${SYSSTRUCT_PROPERTYKEY}(${PKEY_AppUserModel_ID})p.r2'
  System::Call '*${SYSSTRUCT_PROPVARIANT}(${VT_LPWSTR},,&i8 r4)p.r3'
  ${IPropertyStore::SetValue} $1 '($2,$3).r9'
  System::Call 'ole32::CoTaskMemFree(p r4)'
  System::Free $2
  System::Free $3
  ${If} $9 <> 0
    Goto shortcut_failed
  ${EndIf}
  ${IPropertyStore::Commit} $1 '().r9'
  ${IUnknown::Release} $1 ""
  ${If} $9 <> 0
    Goto shortcut_failed
  ${EndIf}
  ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
  ${If} $1 P= 0
    Goto shortcut_failed
  ${EndIf}
  CreateDirectory "$SMPROGRAMS\WriterCraft"
  ${IPersistFile::Save} $1 '("$SMPROGRAMS\WriterCraft\WriterCraft.lnk",1).r9'
  ${IUnknown::Release} $1 ""
  ${IUnknown::Release} $0 ""
  ${If} $9 <> 0
    Goto shortcut_failed
  ${EndIf}
  Return
shortcut_failed:
  SetErrorLevel 1
  Abort "Could not register the WriterCraft Start menu entry."
FunctionEnd
'''
    lines = [header, "OutFile " + nsis_literal(package), "Icon " + nsis_literal(payload / "WriterCraft.ico"),
             "UninstallIcon " + nsis_literal(payload / "WriterCraft.ico"), "VIProductVersion " + nsis_literal(windows_version(metadata["version"])),
             'VIAddVersionKey /LANG=1033 "ProductName" "WriterCraft"',
             'VIAddVersionKey /LANG=1033 "FileDescription" "WriterCraft per-user installer"',
             'VIAddVersionKey /LANG=1033 "LegalCopyright" "WriterCraft contributors"',
             'VIAddVersionKey /LANG=1033 "FileVersion" ' + nsis_literal(metadata["version"]), 'Section "Install"']
    owned = []
    directories = set()
    for source in sorted(payload.rglob("*")):
        if not source.is_file():
            continue
        relative = source.relative_to(payload)
        if any(char in relative.as_posix() for char in '$"\n\r'):
            raise ValueError("Unsafe installed filename")
        parent = str(relative.parent).replace("/", "\\")
        suffix = "" if parent == "." else "\\" + parent
        lines += ['SetOutPath "$INSTDIR' + suffix + '"', 'File ' + nsis_literal(source), 'IfErrors install_failed']
        owned.append(str(relative).replace("/", "\\"))
        directories.update(relative.parents)
    lines += ['WriteUninstaller "$INSTDIR\\Uninstall.exe"', 'IfErrors install_failed', 'Call CreateStartMenu']
    key = 'Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\WriterCraft'
    for name, value in {"DisplayName": "WriterCraft " + metadata["version"], "DisplayVersion": metadata["version"],
                        "Publisher": "WriterCraft contributors", "SourceCommit": metadata["commit"], "ApplicationID": APP_ID,
                        "URLInfoAbout": "https://github.com/avacadorun-dev/writer-craft"}.items():
        lines += ['WriteRegStr HKCU "' + key + '" "' + name + '" ' + nsis_literal(value)]
    lines += ['WriteRegStr HKCU "' + key + '" "InstallLocation" "$INSTDIR"',
              r'''WriteRegStr HKCU "{key}" "DisplayIcon" '$\"$INSTDIR\WriterCraft.ico$\"' '''.replace("{key}", key),
              r'''WriteRegStr HKCU "{key}" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"' '''.replace("{key}", key),
              r'''WriteRegStr HKCU "{key}" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S' '''.replace("{key}", key),
              'WriteRegDWORD HKCU "' + key + '" "NoModify" 1', 'WriteRegDWORD HKCU "' + key + '" "NoRepair" 1',
              'IfErrors install_failed', 'Goto install_done', 'install_failed:', 'SetErrorLevel 1', 'Abort "Could not install WriterCraft program files or registration."', 'install_done:', 'SectionEnd', 'Section "Uninstall"']
    # Explicit owned files only. Unknown files and all nonempty directories survive.
    for name in owned + ["Uninstall.exe"]:
        lines += ['${If} ${FileExists} "$INSTDIR\\' + name + '"', 'ClearErrors',
                  'Delete "$INSTDIR\\' + name + '"', 'IfErrors uninstall_failed', '${EndIf}']
    lines += ['Delete "$SMPROGRAMS\\WriterCraft\\WriterCraft.lnk"', 'RMDir "$SMPROGRAMS\\WriterCraft"']
    for directory in sorted((path for path in directories if path != Path(".")), key=lambda path: len(path.parts), reverse=True):
        lines += ['RMDir "$INSTDIR\\' + str(directory).replace("/", "\\") + '"']
    lines += ['RMDir "$INSTDIR"', 'DeleteRegKey HKCU "' + key + '"', 'Goto uninstall_done',
              'uninstall_failed:', 'SetErrorLevel 1', 'Abort "Could not remove program files. Close WriterCraft and retry uninstall."',
              'uninstall_done:', 'SectionEnd']
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return owned


def build_native_binary(name, output):
    target = TARGETS[name][2]
    run(["cargo", "build", "--locked", "--release", "-p", "studio-desktop", "--features", "editor-preview", "--target", target], cwd=ROOT)
    base = Path(os.environ.get("CARGO_TARGET_DIR", "target"))
    binary = (base if base.is_absolute() else ROOT / base) / target / "release" / ("writercraft.exe" if name == "windows-x64" else "writercraft")
    load_script("package-portable").validate_binary(binary.read_bytes(), target)
    notices = output / "THIRD_PARTY_NOTICES.txt"
    run([sys.executable, ROOT / "scripts/generate-notices.py", "--target", target,
         "--features", "studio-desktop/editor-preview", "--output", notices], cwd=ROOT)
    return binary, notices


def make_deb(payload, staging, metadata, depends):
    executable = staging / "usr/bin/writercraft"
    executable.parent.mkdir(parents=True)
    shutil.copy2(payload / "writercraft", executable)
    executable.chmod(0o755)
    documentation = staging / "usr/share/doc/writercraft"
    documentation.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(payload, documentation, ignore=shutil.ignore_patterns("writercraft"))
    shutil.copy2(payload / "LICENSE", documentation / "copyright")
    launcher = staging / ("usr/share/applications/" + APP_ID + ".desktop")
    launcher.parent.mkdir(parents=True)
    launcher.write_text("[Desktop Entry]\nType=Application\nVersion=1.0\nName=WriterCraft\nComment=Native writing workspace\nExec=writercraft\nIcon=" + APP_ID + "\nTerminal=false\nCategories=Office;WordProcessor;\nStartupNotify=true\nStartupWMClass=" + APP_ID + "\n", encoding="utf-8")
    icon = staging / ("usr/share/icons/hicolor/512x512/apps/" + APP_ID + ".png")
    icon.parent.mkdir(parents=True)
    shutil.copy2(ROOT / "assets/WriterCraft.png", icon)
    control = staging / "DEBIAN/control"
    control.parent.mkdir()
    size = sum(path.stat().st_size for path in staging.rglob("*") if path.is_file())
    control.write_text("Package: writercraft\nVersion: " + debian_version(metadata["version"]) + "\nArchitecture: amd64\nMaintainer: WriterCraft contributors <noreply@writercraft.app>\nSection: editors\nPriority: optional\nInstalled-Size: " + str((size + 1023) // 1024) + "\nDepends: " + ", ".join(sorted(set(depends))) + "\nRecommends: mesa-vulkan-drivers, xdg-desktop-portal-gtk | xdg-desktop-portal-kde | xdg-desktop-portal-gnome\nHomepage: https://github.com/avacadorun-dev/writer-craft\nDescription: Native local writing workspace\n WriterCraft engineering distribution candidate.\n", encoding="utf-8")
    return launcher


def build_linux(metadata, output, package):
    binary, notices = build_native_binary("linux-amd64", output)
    metadata["executable_sha256"] = sha256(binary)
    with tempfile.TemporaryDirectory(prefix="writercraft-deb-", dir=os.environ["RUNNER_TEMP"]) as folder:
        work = Path(folder)
        payload = work / "payload"
        stage_payload(ROOT, binary, payload, metadata, notices)
        (work / "debian").mkdir()
        (work / "debian/control").write_text("Source: writercraft\nSection: editors\nPriority: optional\nMaintainer: WriterCraft contributors <noreply@writercraft.app>\n\nPackage: writercraft\nArchitecture: amd64\nDescription: Native writing workspace\n", encoding="utf-8")
        scanned = capture(["dpkg-shlibdeps", "-O", "-e" + str(binary)], cwd=work)
        if not scanned.startswith("shlibs:Depends="):
            raise ValueError("dpkg-shlibdeps did not produce dependency metadata")
        depends = scanned.removeprefix("shlibs:Depends=").split(", ") + list(LINUX_RUNTIME)
        metadata["runtime_dependencies"] = sorted(set(depends))
        metadata["linux_build_baseline"] = "Debian 12 amd64; glibc 2.36"
        staging = work / "package"
        launcher = make_deb(payload, staging, metadata, depends)
        run(["desktop-file-validate", launcher])
        run(["dpkg-deb", "--root-owner-group", "--build", staging, package])
    metadata["signing"] = "unsigned .deb; integrity supplied by SHA256SUMS"
    metadata["checks"] = {"architecture": True, "notices": True, "application_icon": True, "desktop_launcher": True, "runtime_dependencies": True}


def build_windows(metadata, output, package):
    binary, notices = build_native_binary("windows-x64", output)
    metadata["executable_sha256"] = sha256(binary)
    metadata["windows_file_version"] = windows_version(metadata["version"])
    compiler_root = Path(os.environ["WRITERCRAFT_NSIS"])
    with tempfile.TemporaryDirectory(prefix="writercraft-nsis-", dir=os.environ["RUNNER_TEMP"]) as folder:
        work = Path(folder)
        payload = work / "payload"
        stage_payload(ROOT, binary, payload, metadata, notices)
        icon_from_icns(ROOT / "assets/WriterCraft.icns", payload / "WriterCraft.ico")
        shutil.copy2(compiler_root / "COPYING", payload / "NSIS-COPYING.txt")
        source = work / "WriterCraft.nsi"
        metadata["owned_files"] = nsis_source(payload, metadata, source, package)
        compiler = compiler_root / "makensis.exe"
        compiler_version = capture([compiler, "/VERSION"])
        if compiler_version != "v" + NSIS_VERSION:
            raise ValueError("Unexpected NSIS version: " + compiler_version)
        run([compiler, "/V3", source])
        shutil.copy2(source, output / "installer-source.nsi")
    metadata["signing"] = "unsigned installer; Windows SmartScreen reputation is not established"
    metadata["packaging_tool"] = "NSIS " + NSIS_VERSION + "; zlib compressor"
    metadata["checks"] = {"architecture": True, "notices": True, "application_icon": True}


def check_macos_bundle(app, metadata):
    release = load_script("release-check")
    issues = release.bundle_issues(app, release.workspace_metadata())
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    if issues or info.get("WriterCraftBuildCommit") != metadata["commit"] or info.get("WriterCraftVersion") != metadata["version"]:
        raise ValueError("macOS bundle identity mismatch: " + "; ".join(issues))
    run(["codesign", "--verify", "--deep", "--strict", app])
    notices = app / "Contents/Resources/licenses/THIRD_PARTY_NOTICES.txt"
    if not notices.is_file() or not notices.stat().st_size:
        raise ValueError("macOS dependency notices are missing")
    return sha256(app / "Contents/MacOS/WriterCraft")


def build_macos(metadata, output, package):
    if platform.machine() != "arm64":
        raise ValueError("macOS installer requires an ARM64 runner")
    app = output / "WriterCraft.app"
    environment = dict(os.environ, GITHUB_SHA=metadata["commit"], WRITERCRAFT_EDITOR_PREVIEW="0", WRITERCRAFT_NATIVE_EDITOR="1", WRITERCRAFT_SIGN_IDENTITY="-")
    run(["bash", ROOT / "scripts/package-macos.sh", "--release", "--target", TARGETS["macos-arm64"][2], "--output", app], cwd=ROOT, env=environment)
    metadata["executable_sha256"] = check_macos_bundle(app, metadata)
    with tempfile.TemporaryDirectory(prefix="writercraft-dmg-", dir=os.environ["RUNNER_TEMP"]) as folder:
        work = Path(folder)
        contents = work / "contents"
        contents.mkdir()
        run(["ditto", app, contents / "WriterCraft.app"])
        (contents / "Applications").symlink_to("/Applications")
        run(["hdiutil", "create", "-fs", "HFS+", "-format", "UDZO", "-volname", "WriterCraft " + metadata["version"], "-srcfolder", contents, package])
        run(["hdiutil", "verify", package])
        mount = work / "mounted"
        mount.mkdir()
        run(["hdiutil", "attach", package, "-readonly", "-nobrowse", "-mountpoint", mount])
        try:
            info = plistlib.loads(run(["diskutil", "info", "-plist", mount], stdout=subprocess.PIPE).stdout)
            if info.get("FilesystemType") != "hfs":
                raise ValueError("DMG must mount as an HFS+ filesystem")
            if not (mount / "Applications").is_symlink() or os.readlink(mount / "Applications") != "/Applications":
                raise ValueError("DMG must contain the exact Applications drag target")
            if check_macos_bundle(mount / "WriterCraft.app", metadata) != metadata["executable_sha256"]:
                raise ValueError("Mounted app differs from the built executable")
            if capture([mount / "WriterCraft.app/Contents/MacOS/WriterCraft", "--version"]) != "WriterCraft " + metadata["version"]:
                raise ValueError("Packaged app version does not match source")
        finally:
            run(["hdiutil", "detach", mount])
    metadata["signing"] = "ad-hoc application signature; not Developer ID signed or notarized; unsigned DMG"
    metadata["checks"] = {"architecture": True, "notices": True, "application_icon": True, "dmg_filesystem": True, "applications_link": True, "app_signature": True, "installed_version": True}


def build(name, output):
    require_hosted(name)
    commit = capture(["git", "rev-parse", "HEAD"], cwd=ROOT)
    if commit != os.environ.get("GITHUB_SHA") or not re.fullmatch(r"[a-f0-9]{40}", commit):
        raise ValueError("Hosted checkout must match the exact requested GITHUB_SHA")
    release = load_script("release-check")
    version = release.workspace_metadata()["version"]
    windows_version(version)
    metadata = {"schema_version": 1, "kind": "engineering-distribution-candidate", "version": version, "commit": commit,
                "platform": name, "target": TARGETS[name][2], "source_fingerprint": release.source_fingerprint(),
                "release_ready": False, "platform_ux_qualified": False, "application_id": APP_ID, "cargo_features": ["editor-preview"],
                "build_compiler": capture(["rustc", "--version"])}
    output.mkdir(parents=True, exist_ok=True)
    if any(output.glob("*.dmg")) or any(output.glob("*-setup.exe")) or any(output.glob("*.deb")):
        raise ValueError("Use an empty output directory; existing installer candidates are immutable")
    package = output / ("WriterCraft-" + version + "-" + name + TARGETS[name][3])
    {"macos-arm64": build_macos, "windows-x64": build_windows, "linux-amd64": build_linux}[name](metadata, output, package)
    metadata["package"] = {"filename": package.name, "sha256": sha256(package), "size": package.stat().st_size}
    write_json(output / "distribution-evidence.json", metadata)
    (output / "SHA256SUMS").write_text(metadata["package"]["sha256"] + "  " + package.name + "\n", encoding="utf-8")
    shutil.copy2(ROOT / "docs/distribution/INSTALL.md", output / "INSTALL.md")


def privileged(command):
    return command if os.geteuid() == 0 else ["sudo"] + command


def verify_linux(output):
    require_hosted("linux-amd64")
    evidence_path = output / "distribution-evidence.json"
    metadata = json.loads(evidence_path.read_text())
    package = (output / metadata["package"]["filename"]).resolve()
    if sha256(package) != metadata["package"]["sha256"]:
        raise ValueError("Installer checksum changed before verification")
    for field, expected in {"Package": "writercraft", "Version": debian_version(metadata["version"]), "Architecture": "amd64"}.items():
        if capture(["dpkg-deb", "--field", package, field]) != expected:
            raise ValueError("Incorrect Debian package " + field)
    run(["dpkg-deb", "--info", package], stdout=(output / "deb-info.txt").open("w"))
    run(["dpkg-deb", "--contents", package], stdout=(output / "deb-contents.txt").open("w"))
    base = Path(os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local/share"))) / "WriterCraft"
    base.mkdir(parents=True, exist_ok=True)
    sentinel = base / "distribution-preservation-sentinel.bin"
    if sentinel.exists():
        raise ValueError("Refusing to overwrite an existing preservation sentinel")
    sentinel.write_bytes(b"Synthetic WriterCraft install/remove preservation fixture\n\x00\xff")
    sentinel_hash = sha256(sentinel)
    environment = dict(os.environ, DEBIAN_FRONTEND="noninteractive")
    run(privileged(["apt-get", "install", "-y", package]), env=environment)
    try:
        installed_metadata = Path("/usr/share/doc/writercraft/package-metadata.json")
        installed = json.loads(installed_metadata.read_text())
        if (installed["commit"], installed["version"]) != (metadata["commit"], metadata["version"]):
            raise ValueError("Installed source identity differs from candidate")
        for name in ["LICENSE", "THIRD_PARTY_NOTICES.txt", "copyright"]:
            notice = installed_metadata.parent / name
            if not notice.is_file() or not notice.stat().st_size:
                raise ValueError("Installed dependency/repository notice is missing: " + name)
        if sha256(Path("/usr/bin/writercraft")) != metadata["executable_sha256"]:
            raise ValueError("Installed executable checksum differs from source build")
        if capture(["/usr/bin/writercraft", "--version"]) != "WriterCraft " + metadata["version"]:
            raise ValueError("Installed version differs from source")
        dependencies = capture(["ldd", "/usr/bin/writercraft"])
        (output / "installed-ldd.txt").write_text(dependencies + "\n", encoding="utf-8")
        if "not found" in dependencies:
            raise ValueError("Installed executable has missing linked dependencies")
        run(["desktop-file-validate", "/usr/share/applications/" + APP_ID + ".desktop"])
        if not Path("/usr/share/icons/hicolor/512x512/apps/" + APP_ID + ".png").is_file():
            raise ValueError("Installed application icon is missing")
    finally:
        run(privileged(["apt-get", "remove", "-y", "writercraft"]), env=environment)
    if sha256(sentinel) != sentinel_hash:
        raise ValueError("Removing the package changed user workspace data")
    run(privileged(["apt-get", "purge", "-y", "writercraft"]), env=environment)
    if sha256(sentinel) != sentinel_hash:
        raise ValueError("Purging the package changed user workspace data")
    for path in ["/usr/bin/writercraft", "/usr/share/applications/" + APP_ID + ".desktop", "/usr/share/doc/writercraft/package-metadata.json", "/usr/share/icons/hicolor/512x512/apps/" + APP_ID + ".png"]:
        if Path(path).exists():
            raise ValueError("Uninstall left a packaged file: " + path)
    metadata["checks"].update(unattended_install=True, unattended_uninstall=True, purge=True, user_data_preserved=True, installed_version=True, installed_binary_checksum=True)
    metadata["sentinel_sha256"] = sentinel_hash
    os_release = dict(line.split("=", 1) for line in Path("/etc/os-release").read_text().splitlines() if "=" in line)
    qualification = os_release["ID"].strip('"') + "-" + os_release["VERSION_ID"].strip('"')
    if qualification not in {"debian-12", "ubuntu-24.04"}:
        raise ValueError("Unexpected Linux installation test platform: " + qualification)
    metadata.setdefault("linux_install_tests", {})[qualification] = {"passed": True, "sentinel_sha256": sentinel_hash}
    write_json(evidence_path, metadata)


REQUIRED_CHECKS = {
    "macos-arm64": {"architecture", "notices", "application_icon", "dmg_filesystem", "applications_link", "app_signature", "installed_version"},
    "windows-x64": {"architecture", "notices", "application_icon", "unattended_install", "unattended_uninstall", "user_data_preserved", "unowned_files_preserved", "start_menu", "application_identity", "installed_version", "installed_binary_checksum"},
    "linux-amd64": {"architecture", "notices", "application_icon", "desktop_launcher", "runtime_dependencies", "unattended_install", "unattended_uninstall", "purge", "user_data_preserved", "installed_version", "installed_binary_checksum"},
}


def validate_candidates(downloads):
    candidates = {}
    for path in sorted(downloads.rglob("distribution-evidence.json")):
        metadata = json.loads(path.read_text())
        name = metadata["platform"]
        windows_version(metadata["version"])
        if name not in TARGETS or name in candidates:
            raise ValueError("Unknown or duplicate installer platform")
        if metadata.get("cargo_features") != ["editor-preview"] or metadata.get("application_id") != APP_ID:
            raise ValueError("Installer native editor feature or application identity mismatch")
        if metadata.get("kind") != "engineering-distribution-candidate" or metadata.get("schema_version") != 1 or metadata.get("target") != TARGETS[name][2]:
            raise ValueError("Invalid installer evidence identity")
        if metadata.get("release_ready") is not False or metadata.get("platform_ux_qualified") is not False:
            raise ValueError("Engineering evidence cannot claim release or platform UX qualification")
        if not all(metadata.get("checks", {}).get(key) is True for key in REQUIRED_CHECKS[name]):
            raise ValueError("Required installer acceptance check is missing: " + name)
        filename = metadata["package"]["filename"]
        expected = "WriterCraft-" + metadata["version"] + "-" + name + TARGETS[name][3]
        if filename != expected:
            raise ValueError("Unexpected installer filename")
        package = path.parent / filename
        if package.is_symlink() or not package.is_file() or sha256(package) != metadata["package"]["sha256"] or package.stat().st_size != metadata["package"]["size"]:
            raise ValueError("Installer checksum/size mismatch")
        if (path.parent / "SHA256SUMS").read_text() != metadata["package"]["sha256"] + "  " + filename + "\n":
            raise ValueError("Installer checksum file differs from evidence")
        candidates[name] = (path, metadata)
    if "linux-amd64" in candidates:
        tests = candidates["linux-amd64"][1].get("linux_install_tests", {})
        if not all(tests.get(name, {}).get("passed") is True for name in ["debian-12", "ubuntu-24.04"]):
            raise ValueError("Debian 12 and Ubuntu 24.04 installation tests are required")
    if set(candidates) != set(TARGETS):
        raise ValueError("All three installer platforms are required")
    identities = {(item["version"], item["commit"], item["source_fingerprint"]) for _, item in candidates.values()}
    if len(identities) != 1:
        raise ValueError("Installers must have the same exact source and version")
    version, commit, fingerprint = identities.pop()
    windows_version(version)
    if not re.fullmatch(r"[a-f0-9]{40}", commit) or not re.fullmatch(r"[a-f0-9]{64}", fingerprint):
        raise ValueError("Invalid candidate source identity")
    return candidates


def collect(downloads, output):
    candidates = validate_candidates(downloads)
    if output.exists():
        raise ValueError("Use a fresh combined output directory")
    output.mkdir(parents=True)
    sums = []
    for name, (path, metadata) in candidates.items():
        package = metadata["package"]
        shutil.copy2(path.parent / package["filename"], output / package["filename"])
        shutil.copy2(path, output / (name + "-evidence.json"))
        sums.append(package["sha256"] + "  " + package["filename"])
    (output / "SHA256SUMS").write_text("\n".join(sorted(sums)) + "\n", encoding="utf-8")
    write_json(output / "distribution-manifest.json", {"schema_version": 1, "kind": "engineering-distribution-candidate", "release_ready": False, "platforms": {name: metadata for name, (_, metadata) in candidates.items()}})
    shutil.copy2(ROOT / "docs/distribution/INSTALL.md", output / "INSTALL.md")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    builder = subparsers.add_parser("build")
    builder.add_argument("--platform", choices=sorted(TARGETS), required=True)
    builder.add_argument("--output", type=Path, required=True)
    verifier = subparsers.add_parser("verify-linux")
    verifier.add_argument("--output", type=Path, required=True)
    collector = subparsers.add_parser("collect")
    collector.add_argument("--downloads", type=Path, required=True)
    collector.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "build":
        build(args.platform, args.output.resolve())
    elif args.command == "verify-linux":
        verify_linux(args.output.resolve())
    else:
        collect(args.downloads.resolve(), args.output.resolve())


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print("Installer qualification failed: " + str(error), file=sys.stderr)
        sys.exit(1)
