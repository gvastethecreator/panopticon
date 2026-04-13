$base = "https://cdn.hugeicons.com/icons"
$dir = "d:\DEV\panopticon\assets\ui-icons"

$icons = @(
    @{ file = "option-monitor";      name = "monitor-01" },
    @{ file = "option-tag";          name = "tag-01" },
    @{ file = "option-group";        name = "layers-02" },
    @{ file = "option-palette";      name = "paint-brush-01" },
    @{ file = "option-menu";         name = "menu-01" },
    @{ file = "option-exit";         name = "logout-03" },
    @{ file = "option-paint";        name = "paint-bucket" },
    @{ file = "option-image-fit";    name = "crop-01" },
    @{ file = "option-cycle";        name = "arrow-reload-horizontal" },
    @{ file = "option-profile-edit"; name = "user-edit-01" }
)

$fallbacks = @{
    "option-monitor"      = @("monitor-02","screen","computer")
    "option-tag"          = @("tag-02","label","bookmark-02")
    "option-group"        = @("group-items","object-01","collection-01")
    "option-palette"      = @("palette","color-swatch","brush-01")
    "option-menu"         = @("menu-02","list-view","align-left")
    "option-exit"         = @("logout-01","door-01","sign-out")
    "option-paint"        = @("paint-bucket-02","paint-01","fill-color")
    "option-image-fit"    = @("expand-01","scale-01","resize-01")
    "option-cycle"        = @("refresh","rotate-01","arrow-refresh-01")
    "option-profile-edit" = @("user-edit-02","edit-user-01","user-02")
}

function Download-Icon($url, $outPath) {
    $r = Invoke-WebRequest $url -UseBasicParsing -ErrorAction Stop
    if ($r.Content.Length -lt 100) { throw "too small: $($r.Content.Length)" }
    $svg = $r.Content -replace 'stroke="#[0-9A-Fa-f]{3,6}"','stroke="currentColor"' -replace 'fill="#[0-9A-Fa-f]{3,6}"','fill="currentColor"'
    Set-Content $outPath $svg -Encoding UTF8 -NoNewline
    return $r.Content.Length
}

foreach ($icon in $icons) {
    $outPath = Join-Path $dir "$($icon.file).svg"
    $url = "$base/$($icon.name)-stroke-rounded.svg"
    $ok = $false
    try {
        $sz = Download-Icon $url $outPath
        Write-Host "OK: $($icon.file).svg <- $($icon.name) ($sz b)"
        $ok = $true
    } catch {
        Write-Host "FAIL primary: $($icon.file) <- $($icon.name): $($_.Exception.Message)"
    }
    if (-not $ok -and $fallbacks.ContainsKey($icon.file)) {
        foreach ($fb in $fallbacks[$icon.file]) {
            try {
                $sz = Download-Icon "$base/$fb-stroke-rounded.svg" $outPath
                Write-Host "  FALLBACK OK: $($icon.file).svg <- $fb ($sz b)"
                $ok = $true
                break
            } catch {}
        }
    }
    if (-not $ok) { Write-Host "MISSING: $($icon.file)" }
}

Write-Host "`n--- Final listing ---"
Get-ChildItem $dir -Filter "option-*.svg" | Sort-Object Name | Select-Object Name,Length
