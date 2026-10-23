$names = [ordered]@{
    InboxIcon = 'inbox.svg'
    WeekIcon = 'calendar-01.svg'
    NextWeekIcon = 'calendar-02.svg'
    SomedayIcon = 'clock-01.svg'
    LogbookIcon = 'book-open.svg'
    StatisticsIcon = 'analytics-01.svg'
    SettingsIcon = 'sliders.svg'
    AboutIcon = 'calendar-01.svg'
    TrashIcon = 'delete.svg'
    ProjectIcon = 'folder.svg'
    PlusIcon = 'plus.svg'
    MoreIcon = 'more-h.svg'
    CheckIcon = 'check.svg'
}
$lines = foreach ($entry in $names.GetEnumerator()) {
    $svg = Get-Content (Join-Path $PSScriptRoot "assets/$($entry.Value)") -Raw
    $encoded = [Uri]::EscapeDataString($svg)
    "export const $($entry.Key) = ``data:image/svg+xml,$encoded``;"
}
Set-Content -Encoding utf8 (Join-Path $PSScriptRoot 'icons.ts') $lines

