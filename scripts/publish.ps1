[CmdletBinding()]
param(
    [switch]$DryRun,
    [switch]$ListOnly,
    [switch]$Yes,
    [switch]$AllowDirty,
    [switch]$NoWait,
    [int]$WaitSeconds = 180,
    [string[]]$Exclude = @('sequoia-test', 'sequoia-seq-lsp')
)

function Wait-ForCrate {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Version,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        & cargo info "$Name@$Version" *> $null
        if ($LASTEXITCODE -eq 0) {
            return
        }
        Start-Sleep -Seconds 5
    }
    throw "Timed out waiting for $Name $Version to appear in the crates.io index."
}

$workspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$manifestPath = Join-Path $workspaceRoot 'Cargo.toml'
$cratesRoot = Join-Path $workspaceRoot 'crates'

$metadataJson = & cargo metadata --manifest-path $manifestPath --no-deps --format-version 1
if ($LASTEXITCODE -ne 0) {
    throw 'cargo metadata failed.'
}
$metadata = $metadataJson | ConvertFrom-Json

$packages = @($metadata.packages | Where-Object {
    $_.manifest_path.StartsWith($cratesRoot, [System.StringComparison]::OrdinalIgnoreCase) -and
    $Exclude -notcontains $_.name
})

if ($packages.Count -eq 0) {
    throw 'No publishable crates were found.'
}

$versions = @($packages | ForEach-Object { $_.version } | Sort-Object -Unique)
if ($versions.Count -ne 1) {
    throw "Workspace crates have different versions: $($versions -join ', ')."
}
$version = $versions[0]

$byName = @{}
foreach ($package in $packages) {
    $byName[$package.name] = $package
}

$dependencies = @{}
foreach ($package in $packages) {
    $dependencies[$package.name] = @(
        $package.dependencies |
            Where-Object { $byName.ContainsKey($_.name) } |
            ForEach-Object { $_.name } |
            Sort-Object -Unique
    )
}

$remaining = @{}
foreach ($name in $byName.Keys) {
    $remaining[$name] = $dependencies[$name].Count
}

$ordered = New-Object System.Collections.Generic.List[string]
while ($remaining.Count -gt 0) {
    $ready = @($remaining.Keys | Where-Object { $remaining[$_] -eq 0 } | Sort-Object)
    if ($ready.Count -eq 0) {
        throw 'The workspace dependency graph contains a cycle.'
    }
    foreach ($name in $ready) {
        [void]$ordered.Add($name)
        $remaining.Remove($name)
        foreach ($other in @($remaining.Keys)) {
            if ($dependencies[$other] -contains $name) {
                $remaining[$other] = $remaining[$other] - 1
            }
        }
    }
}

Write-Host "Sequoia crates to publish (version $version):"
$index = 1
foreach ($name in $ordered) {
    Write-Host ("  {0,2}. {1}" -f $index, $name)
    $index++
}

if ($ListOnly) {
    return
}

if (-not $AllowDirty) {
    $dirty = & git -C $workspaceRoot status --porcelain --untracked-files=no
    if ($LASTEXITCODE -ne 0) {
        throw 'git status failed.'
    }
    if ($dirty) {
        throw 'The working tree has uncommitted changes. Commit or stash them, or pass -AllowDirty.'
    }
}

if (-not $DryRun -and -not $Yes) {
    $answer = Read-Host "Publish $($ordered.Count) crates to crates.io? [y/N]"
    if ($answer -notmatch '^(y|yes)$') {
        Write-Host 'Aborted.'
        return
    }
}

if (-not $DryRun) {
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
    $credentials = Join-Path $cargoHome 'credentials.toml'
    if (-not $env:CARGO_REGISTRY_TOKEN -and -not (Test-Path $credentials)) {
        Write-Warning 'No CARGO_REGISTRY_TOKEN or cargo credentials file was found. Run cargo login first.'
    }
}

$published = New-Object System.Collections.Generic.List[string]
$skipped = New-Object System.Collections.Generic.List[string]
$preflightSkipped = New-Object System.Collections.Generic.List[string]

foreach ($name in $ordered) {
    $crateVersion = $byName[$name].version
    Write-Host ''
    Write-Host "==> $name $crateVersion"

    $cargoArgs = @('publish', '--manifest-path', $manifestPath, '-p', $name)
    if ($DryRun) {
        $cargoArgs += @('--dry-run', '--no-verify')
    }
    if ($AllowDirty) {
        $cargoArgs += '--allow-dirty'
    }

    $output = & cargo @cargoArgs 2>&1 | ForEach-Object {
        if ($_ -is [System.Management.Automation.ErrorRecord]) {
            $_.Exception.Message
        } else {
            [string]$_
        }
    }
    $exitCode = $LASTEXITCODE
    foreach ($line in $output) {
        Write-Host $line
    }

    if ($exitCode -ne 0) {
        $text = ($output | Out-String)
        if ($text -match 'already uploaded|already exists|crate version .* already') {
            Write-Warning "$name $crateVersion is already published; skipping."
            [void]$skipped.Add($name)
            continue
        }
        if ($DryRun -and ($text -match 'failed to select a version for the requirement' -or $text -match 'no matching package named')) {
            Write-Warning "$name preflight skipped: its local dependencies are not published yet."
            [void]$preflightSkipped.Add($name)
            continue
        }
        throw "cargo publish failed for $name (exit code $exitCode)."
    }

    [void]$published.Add($name)

    if (-not $DryRun -and -not $NoWait) {
        Write-Host "Waiting for $name $crateVersion in the crates.io index..."
        Wait-ForCrate -Name $name -Version $crateVersion -TimeoutSeconds $WaitSeconds
    }
}

Write-Host ''
if ($published.Count -gt 0) {
    if ($DryRun) {
        Write-Host "Dry-run succeeded: $($published -join ', ')"
    } else {
        Write-Host "Published: $($published -join ', ')"
    }
}
if ($skipped.Count -gt 0) {
    Write-Host "Skipped (already published): $($skipped -join ', ')"
}
if ($preflightSkipped.Count -gt 0) {
    Write-Host "Preflight skipped: $($preflightSkipped -join ', ')"
}
if (-not $DryRun) {
    Write-Host 'Done.'
}
