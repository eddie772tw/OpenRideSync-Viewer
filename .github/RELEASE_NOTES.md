OpenRideSync Viewer 的首個公開版本，適用於 Windows 10/11 x64。

- 車型圖鑑、車型圖片、歷史旅程與預設載入的 OpenStreetMap 地圖。
- 系統瀏覽器 Google 登入、Windows 加密的持久登入、離線快照瀏覽。
- 車型與旅程並行下載及各自的進度環。
- 帳號資訊、選擇性清除快取、登出及清除所有本機資料。
- 隨視窗寬度延展的介面與容器內獨立捲動。

下載 `OpenRideSync-Viewer-0.1.0-windows-x64.zip`，解壓縮後執行 `OpenRideSync Viewer.exe`。需要 Microsoft Edge WebView2 Runtime。

**首次使用：正式套件已包含服務用戶端設定，直接使用原本 RoadSync 的同一個 Google 帳號登入即可。** 設定由 GitHub Actions Secrets 在編譯時注入，不寫入原始碼；服務金鑰可從執行檔取出，這不等同於秘密儲存。套件不含任何個人登入權杖、帳號資訊、車型圖片或旅程快照。

### 服務相容性與金鑰版本備查

| 項目 | 版本／指紋 |
| --- | --- |
| 參考 APK | Honda RoadSync `26.8.10`，versionCode `309` |
| 套件 | `com.honda.ms.dm.sab` |
| base.apk SHA-256 | `94facfbed5044856dc65173b5a287a25418c301ca192fcd59722342c9a004cbb` |
| 金鑰識別 | `roadsync-26.8.10-309`（本專案命名，非原廠版號） |
| Honda API key SHA-256 | `ed229bfb2910fcf4540967b94fa6255f268183f126891b4ec2669d205bbf0fd8` |
| Firebase API key SHA-256 | `d3b040469ae2e7122601a339253a1d1ee3213c12e6f92a336149f40125f31b22` |

APK 僅作為相容性與來源版本參照，不包含於發布檔。工作流程會檢查注入金鑰與以上指紋相符；不在日誌或 Release Notes 顯示金鑰明文。

僅讀取既有資料；不新增、上傳、修改或刪除雲端旅程。此版本尚未進行 Windows 程式碼簽章。CI 使用合成測試，不登入真實帳號。既有原生登入與讀取的本機驗證不等於手機實車上傳驗證。

套件含 MIT 授權、第三方聲明及 SHA-256 校驗檔。Google、Honda 及圖磚服務需要網路，服務介面變更可能影響相容性。

**Disclaimer:** This project is an independent open-source implementation and is **NOT** affiliated with, endorsed by, or in any way officially connected with Honda Motor Co., Ltd. "Honda" and "RoadSync" are registered trademarks of Honda Motor Co., Ltd.
