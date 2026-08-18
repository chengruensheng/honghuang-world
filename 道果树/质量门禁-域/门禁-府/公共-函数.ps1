# 找工作区根：从脚本目录向上探测含 Cargo.toml 的目录。
function 找工作区根 {
    $当前 = Split-Path -Parent $PSScriptRoot
    while ($当前 -and -not (Test-Path (Join-Path $当前 'Cargo.toml'))) {
        $当前 = Split-Path -Parent $当前
    }
    return $当前
}

# 获取代码源文件清单（排除 构建物/记忆/临时/版本库）。
function 取源码文件([string]$根, [string]$过滤) {
    Get-ChildItem $根 -Recurse -File -Filter $过滤 | Where-Object {
        $_.FullName -notmatch '\\道果树\\|\\\.上下文\\|\\临时文件夹\\|\\\.git\\|\\deps\\|\\incremental\\'
    }
}
