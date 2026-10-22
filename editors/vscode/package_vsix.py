#!/usr/bin/env python3
"""把本目录打成 Cursor / VS Code 能安装的 VSIX。不依赖 npm。"""

import json
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SKIP = {"test", ".vscode", "package_vsix.py"}


def main() -> None:
    package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
    name = package["name"]
    version = package["version"]
    publisher = package["publisher"]
    out = ROOT / f"{name}-{version}.vsix"
    files = [
        path
        for path in ROOT.rglob("*")
        if path.is_file() and not any(part in SKIP for part in path.relative_to(ROOT).parts)
        and path.suffix != ".vsix"
    ]
    manifest = f"""<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011" xmlns:d="http://schemas.microsoft.com/developer/vsx-schema-design/2011">
  <Metadata>
    <Identity Language="en-US" Id="{name}" Version="{version}" Publisher="{publisher}" />
    <DisplayName>{package["displayName"]}</DisplayName>
    <Description xml:space="preserve">{package["description"]}</Description>
    <Categories>{",".join(package.get("categories", []))}</Categories>
    <GalleryFlags>Public</GalleryFlags>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="{package["engines"]["vscode"]}" />
      <Property Id="Microsoft.VisualStudio.Code.ExtensionKind" Value="workspace" />
      <Property Id="Microsoft.VisualStudio.Services.GitHubFlavoredMarkdown" Value="true" />
    </Properties>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code" />
  </Installation>
  <Dependencies />
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true" />
    <Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true" />
  </Assets>
</PackageManifest>
"""
    content_types = """<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension=".json" ContentType="application/json" />
  <Default Extension=".js" ContentType="application/javascript" />
  <Default Extension=".md" ContentType="text/markdown" />
  <Default Extension=".vsixmanifest" ContentType="text/xml" />
</Types>
"""
    with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("[Content_Types].xml", content_types)
        archive.writestr("extension.vsixmanifest", manifest)
        for path in files:
            archive.write(path, f"extension/{path.relative_to(ROOT).as_posix()}")
    print(out)


if __name__ == "__main__":
    main()
