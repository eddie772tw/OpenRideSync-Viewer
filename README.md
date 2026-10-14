# OpenRideSync Viewer

[![CI](https://github.com/eddie772tw/OpenRideSync-Viewer/actions/workflows/ci.yml/badge.svg)](https://github.com/eddie772tw/OpenRideSync-Viewer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/eddie772tw/OpenRideSync-Viewer)](https://github.com/eddie772tw/OpenRideSync-Viewer/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Windows 桌面上的 RoadSync 車型與騎乘收藏。以 Tauri 2、Rust、React 與 TypeScript 實作，將車型圖鑑、歷史旅程及路線地圖放在同一個本機介面。介面使用繁體中文，時間以 Asia/Taipei 顯示。

**Disclaimer:** This project is an independent open-source implementation and is **NOT** affiliated with, endorsed by, or in any way officially connected with Honda Motor Co., Ltd. "Honda" and "RoadSync" are registered trademarks of Honda Motor Co., Ltd.

## 功能

- 車型圖片、年份與車碼搜尋，從車型對照旅程，也能從旅程回看相關車型。
- 歷史旅程的日摘要、分段時間、距離、速度、GPS 路線與起終點。
- Leaflet 預設載入 OpenStreetMap 底圖；路線在本機繪製。
- 車型與旅程並行下載，以兩個進度環呈現實際完成數，完成後顯示準備就緒。
- 系統瀏覽器 Google 登入；登入狀態由 Windows DPAPI 加密保存。
- 授權失效時提示重新登入；已下載的資料仍可瀏覽。
- 帳號管理顯示 Google 帳號與 RoadSync ID，可清除圖片或旅程快取、登出或清除所有本機資料。
- 隨視窗寬度延展的版面，旅程列表與詳情各自捲動。

車碼使用精確值比對；缺失或共用車碼的資訊會保留不確定性，不推測年份。

## 安裝與首次使用

支援 **Windows 10/11 x64**，需要 [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)。目前提供可攜式 ZIP，沒有安裝程式；執行檔尚未進行程式碼簽章。macOS、Linux、Windows ARM64 尚未支援。

1. 從 [Releases](https://github.com/eddie772tw/OpenRideSync-Viewer/releases) 下載 Windows x64 ZIP 與 `.sha256`。
2. 解壓縮，執行 `OpenRideSync Viewer.exe`。
3. 使用原本 RoadSync 的同一個 Google 帳號登入；正式 Release 已帶入服務用戶端設定，不需手動輸入金鑰。
4. 等待車型與歷史旅程下載完成，開啟旅程分頁。

正式 Release 的服務設定由 GitHub Actions Secret 在編譯時注入，原始碼與 Git 歷史不包含金鑰。**執行檔中的服務金鑰可以被取出，並非秘密儲存機制。** 個人的 Google 權杖、帳號 ID 與旅程資料永遠不隨套件發布。

### 自行編譯的服務設定

一般 CI 與預設自行編譯不注入金鑰，首次使用可自行匯入。先複製 [service-config.example.json](service-config.example.json)，填入既有 RoadSync 服務的 Firebase 用戶端設定與服務 API key：

```json
{
  "firebase": {
    "apiKey": "YOUR_FIREBASE_API_KEY",
    "authDomain": "YOUR_FIREBASE_AUTH_DOMAIN",
    "projectId": "YOUR_FIREBASE_PROJECT_ID"
  },
  "hondaApiKey": "YOUR_SERVICE_API_KEY"
}
```

設定檔必須由使用者自行合法取得並有權使用；本專案不提供金鑰申請或擷取工具，也不繞過服務授權。建立另一個 Firebase 專案無法存取既有 RoadSync 帳號。不要填入 Google 密碼、refresh token、帳號 ID 或實車測量資料。匯入前會驗證格式；設定僅保存於本機，登入中無法更換設定。若需更換服務設定，先使用帳號管理的「清除所有資料」重新開始。

下載檔案可用 PowerShell 核對 SHA-256，與旁邊的 `.sha256` 檔內容比較：

```powershell
Get-FileHash '.\OpenRideSync-Viewer-0.1.0-windows-x64.zip' -Algorithm SHA256
```

## 隱私與唯讀範圍

程式僅允許 `ListModelCatalogs`、`ListModelSeries`、`ListModels`、`GetImageBinary`、`ListDailyTrips`、`ListTripDetails` 六種讀取 RPC。沒有新增、上傳、修改或刪除雲端旅程的命令。帳號管理的刪除操作僅作用於 Viewer 的本機資料。

- Google 登入於系統瀏覽器完成。回呼使用隨機路徑、單次 nonce、來源檢查；Rust 驗證 Firebase JWT 的簽章、專案、有效期與 Google 登入來源。
- refresh token 與帳號資訊透過 Windows DPAPI CurrentUser 加密；短效 ID token 不落盤。每次啟動或更新會嘗試續期，離線不等於登入失效。
- 車型與旅程快照保存於本機，**快照本身不加密**，其中包含私人路線。請勿上傳整個資料目錄或將快照附在公開 issue。
- 更新驗證完整後才切換快照，失敗會保留上次成功資料。清除圖片／旅程也會移除舊快照中的對應資料；登出保留快照；清除所有資料會移除登入、快照及匯入的服務設定。
- 地圖需要網路。OpenStreetMap 圖磚服務會收到 IP 與顯示範圍；不會收到整份旅程或帳號資訊。本專案不預抓離線圖磚、不呼叫路徑規劃或地理編碼服務。
- 沒有內建遙測、廣告或自動更新。Google、Honda 與 OpenStreetMap 的服務由各自供應者提供。

資料位於 `%APPDATA%\lab.roadsync.viewer`：

| 檔案 | 用途 |
| --- | --- |
| `service-config.json` | 可選的本機設定，優先於編譯時注入的設定；不納入 Git |
| `session.bin` | Windows 加密的登入狀態與帳號資訊 |
| `snapshot-*.json` | 車型、圖片與歷史旅程快照 |
| `latest.json` | 最後成功快照的指標與 SHA-256 |
| `last-operation.json` | 操作時間、狀態與錯誤代碼，不含權杖、帳號 ID 或路線 |

`lab.roadsync.viewer` 與內部 `open-roadsync-viewer` 套件名稱保留早期版本的儲存相容性，公開產品名稱統一為 **OpenRideSync Viewer**。正式 Release 可沿用早期版本的加密登入與快照；未注入服務設定的自行編譯版本則須先匯入自己的設定。

## 自行編譯

需要 Node.js 24、Python 3.11+、Rustup、Visual Studio 2022 C++ Build Tools（MSVC 與 Windows SDK）及 WebView2 Runtime。Rust 1.96.0、rustfmt 與 Clippy 由 `rust-toolchain.toml` 固定。

```powershell
git clone https://github.com/eddie772tw/OpenRideSync-Viewer.git
cd OpenRideSync-Viewer
.\build.cmd
```

`build.ps1` 依序進行公開原始碼與版本檢查、`npm ci`、前端測試、Rust 格式／測試／Clippy、TypeScript 檢查與 Tauri release 編譯，再收集第三方授權並產生：

```text
release/
  OpenRideSync Viewer.exe
  OpenRideSync Viewer.exe.sha256
  OpenRideSync-Viewer-0.1.0-windows-x64.zip
  OpenRideSync-Viewer-0.1.0-windows-x64.zip.sha256
```

預設建置不需要服務金鑰，也不讀取使用者登入或實車資料；未配置的版本在執行時匯入設定。可用 `build.ps1 -ServiceConfig C:\private\service-config.json` 或環境變數 `OPENRIDESYNC_SERVICE_CONFIG`（JSON）在編譯時注入。設定檔只接受服務用戶端欄位，不接受登入權杖。`-DebugBuild` 產生 `release/debug`；`-SkipChecks` 僅略過測試與 Rust lint，仍執行公開檔案檢查、TypeScript 檢查與編譯。編譯前請關閉使用這個資料夾的 Vite 伺服器與輸出目錄中的執行檔。

## 開發與測試

```powershell
npm ci
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri -- dev
```

一般測試全部使用合成資料，涵蓋登入回呼、DPAPI、設定匯入、唯讀 RPC 白名單、資料與座標驗證、車型／日期篩選及本機快取清除，不連線登入真實帳號。

使用 `npm run dev` 開啟 http://localhost:1420 可檢查導入版面。`?setup` 顯示設定匯入，`?onboarding` 顯示登入階段，`?onboarding=download` 顯示雙進度環。瀏覽器預覽不執行原生登入或本機資料刪除。

已有相容的私人展示頁時，可明確建立本機預覽資料：

```powershell
python scripts/prepare-preview.py C:\private\gallery\index.html
$env:ROADSYNC_PREVIEW = '1'
npm run dev
```

`.preview` 受 Git 忽略，亦不包含於 release。`?onboarding=ready`、`?session=expired`、`?session=offline` 可搭配快照檢查完成與通知狀態。不要將個人車型圖片、旅程座標或帳號資訊加入公開測試 fixture。

另有兩項預設忽略的研究驗證：設定 `ROADSYNC_VERIFIED_RUN` 可離線重播相容的私人 RPC 回應；設定 `ROADSYNC_VIEWER_STATE` 可明確執行 `live_parallel_read_and_account_identity` 唯讀測試。它會續期指定資料目錄內的登入，必須由使用者另行啟用；CI 不執行這些測試。原生登入、雲端讀取、UI 預覽與實車驗證各自代表不同證據，本 Viewer 不宣稱已驗證手機實車上傳。

## CI 與 Release

- [CI](.github/workflows/ci.yml)：push `main`、pull request 與手動觸發；Windows runner 執行完整檢查與 build，保留可攜式 ZIP 和校驗檔。
- [Release](.github/workflows/release.yml)：push `V*` 或 `v*` tag 時重用 CI。tag 必須等於各版本檔案的版本；先驗證注入設定符合 `service-provenance.json` 的金鑰指紋，成功後驗證 ZIP 校驗碼、建立 draft、附上資產，再發布正式 Release。只有發佈 job 取得 `contents: write`。
- GitHub Actions 以完整 commit SHA 固定，Dependabot 每週檢查 npm、Cargo 與 Actions 更新。一般 CI 不使用服務設定或個人帳號 secrets。Release 必須設定 repository secret `ROADSYNC_SERVICE_CONFIG`，內容為上方格式的 JSON；只由 tag 的 Release workflow 傳入可重用建置 job，未提供或指紋不符即停止發布。

服務相容性基線為 **RoadSync 26.8.10 / versionCode 309**（`com.honda.ms.dm.sab`）；金鑰識別為本專案指定的 `roadsync-26.8.10-309`。APK SHA-256 與服務／Firebase key 指紋見 [service-provenance.json](service-provenance.json) 與 Release Notes。此識別不是原廠正式的金鑰版號。

維護者發佈時先同步 `package.json`、npm lockfile、Cargo manifest/lock、Tauri 設定、側欄版本、README 範例及 `.github/RELEASE_NOTES.md`，若更換設定也更新 `service-provenance.json`，待 `main` CI 通過後：

```powershell
git tag -a V0.1.0 -m 'OpenRideSync Viewer V0.1.0'
git push origin V0.1.0
```

失敗可於 Actions 重新執行；公開完成的 Release 不會被工作流程覆寫。手動執行 Release 時須選擇既有版本 tag。

## 授權與貢獻

本專案採用 [MIT License](LICENSE)。第三方元件、OpenStreetMap 資料、Honda 車型圖片與商標仍由各權利人持有；詳見 [第三方聲明](THIRD-PARTY-NOTICES.md) 與可攜式套件內的 `THIRD-PARTY-LICENSES.txt`。

歡迎透過 issue 回報重現步驟、Windows 版本與不含個資的錯誤代碼。提交 PR 前執行 `build.cmd`；請勿附上服務設定、憑證、HAR、APK、帳號 ID、私人快照或精確位置。
