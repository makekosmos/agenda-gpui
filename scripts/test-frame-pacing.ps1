param([string]$CoreTest = 'test_pending_present_defers_draw_but_not_callbacks')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$downloaded = @()
$revision = 'd89e9c2124b2786a390c7a451c7488601b4da2e1'
Push-Location $root
try {
    # The published GPUI snapshot omits the upstream test fonts.
    foreach ($font in @('ibm-plex-sans/IBMPlexSans-Regular.ttf', 'lilex/Lilex-Regular.ttf')) {
        $path = Join-Path $root "assets/fonts/$font"
        if (!(Test-Path -LiteralPath $path)) {
            $downloaded += $path
            rtk proxy curl.exe -fLsS --connect-timeout 5 --max-time 30 --create-dirs -o $path "https://raw.githubusercontent.com/zed-industries/zed/$revision/assets/fonts/$font"
            if ($LASTEXITCODE -ne 0) { throw "Could not fetch upstream test font: $font" }
        }
    }
    rtk cargo test -p agenda-gpui -p gpui-pre --features gpui/test-support --lib $CoreTest
    $result = $LASTEXITCODE
    if ($result -eq 0 -and $CoreTest -ne 'test_cached_window_controls_survive_repaint') {
        rtk cargo test -p agenda-gpui -p gpui-pre --features gpui/test-support --lib test_cached_window_controls_survive_repaint
        $result = $LASTEXITCODE
    }
    if ($result -eq 0) {
        rtk cargo test -p agenda-gpui -p gpui-pre-windows --features gpui/test-support --lib test_frame_pacing_
        $result = $LASTEXITCODE
    }
} finally {
    foreach ($path in $downloaded) {
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path }
    }
    Pop-Location
}
exit $result
