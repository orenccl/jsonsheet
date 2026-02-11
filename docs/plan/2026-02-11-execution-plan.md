# JsonSheet 改進執行計畫（2026-02-11）

## 目標
- 提升資料量較大時的互動效能與記憶體可控性。
- 補齊回歸風險（E2E 選擇器、i18n、DOM id、寫檔安全性）。
- 完成可驗證交付：測試與 lint 全綠。

## 執行步驟
- [x] Step 1: 修正 UI E2E 選擇器與空畫面識別（`#empty-state`）。
- [x] Step 2: 補齊 `zh-Hant` 字串鍵，新增 i18n key parity 測試。
- [x] Step 3: 解決 `sanitize_id` 衝突風險（加入穩定 hash）。
- [x] Step 4: JSON / JSheet 改為原子寫入（temp file + rename）。
- [x] Step 5: 優化計算列渲染，移除每列重算欄位集合。
- [x] Step 6: 修正編輯時 Enter/Tab 導航不同步（傳入正確 `visible_rows`）。
- [x] Step 7: Undo/Redo 歷史加上上限，避免無界成長。
- [x] Step 8: 公式解析支援 bracket 欄名（例如 `[總分]`）。
- [x] Step 9: 全量驗證（`cargo fmt --check`、`cargo clippy --all-targets --all-features`、`cargo test`）。

## 驗收標準
- 功能行為不回退，既有測試維持通過。
- 新增測試覆蓋：i18n parity、ID 穩定性、公式 bracket 欄名、歷史上限。
- 文件與程式碼同步更新。


