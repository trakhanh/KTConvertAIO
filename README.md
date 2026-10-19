<p align="center">
  <img src="src-tauri/icons/128x128.png" width="96" alt="KT Convert" />
</p>

<h1 align="center">KT Convert</h1>

<p align="center">
  Đổi định dạng file ngay trong File Explorer — không cần mở app, không cần upload.<br />
  Chọn file, nhấn <b>Ctrl+Alt+C</b>, bấm định dạng. Xong.
</p>

<p align="center">
  <a href="https://github.com/trakhanh/KTConvertAIO/releases/latest/download/KTConvert-0.1.0-Setup.exe"><b>⬇ Tải bộ cài cho Windows (57 MB)</b></a>
  &nbsp;·&nbsp;
  <a href="https://github.com/trakhanh/KTConvertAIO/releases">Các phiên bản</a>
</p>

<p align="center"><b>Tác giả:</b> Khánh Trà</p>

---

KT Convert chạy nền ở khay hệ thống. Khi cần đổi định dạng, một vòng tròn phong cách kính mờ (liquid glass) hiện ngay dưới con trỏ chuột để bạn chọn định dạng đích. Mọi việc xử lý diễn ra **ngay trên máy bạn** bằng FFmpeg: file không bị gửi đi đâu cả. Giao diện có **tiếng Việt** và tiếng Anh.

## Tải và cài đặt

1. Tải [**KTConvert-0.1.0-Setup.exe**](https://github.com/trakhanh/KTConvertAIO/releases/latest/download/KTConvert-0.1.0-Setup.exe).
2. Mở file vừa tải. Nếu Windows hiện cảnh báo **"Windows protected your PC"** (do app chưa có chữ ký số), bấm **More info → Run anyway**.
3. Chọn ngôn ngữ, bấm cài. Bộ cài chỉ cài cho tài khoản của bạn, **không cần quyền admin**, và đã kèm sẵn FFmpeg — không phải cài thêm gì.
4. Sau khi cài, biểu tượng KT Convert nằm ở khay hệ thống (góc phải dưới, có thể ẩn trong mũi tên **^**).

**Yêu cầu:** Windows 10 hoặc 11, 64-bit.

## Cách dùng

| Cách | Thao tác |
| --- | --- |
| **Phím tắt** | Chọn một hoặc nhiều file trong File Explorer hoặc trên desktop, nhấn **Ctrl+Alt+C**. Bấm vào định dạng muốn đổi (hoặc phím số 1–8, phím mũi tên + Enter). **Esc** để huỷ. |
| **Kéo file + Shift** | Đang kéo một file, nhấn **Shift**: vòng tròn hiện dưới con trỏ. Thả file lên định dạng muốn đổi; thả vào giữa vòng để huỷ. |
| **Menu khay** | Bấm biểu tượng KT Convert → *Chuyển đổi file đang chọn*. |

- File mới được lưu **ngay cạnh file gốc**: `anh.png` → `anh.jpg` (trùng tên thì thành `anh (1).jpg`).
- Mặc định **giữ nguyên file gốc**. Muốn tự xoá file gốc sau khi đổi, bấm nút **Giữ file gốc / Xóa file gốc** ở giữa vòng tròn (hoặc phím **D**). File gốc được chuyển vào **Thùng rác**, lấy lại được.
- Thẻ thông báo góc phải dưới hiện tiến độ; bấm **Mở thư mục** để tới file vừa tạo.

## Cài đặt

Chuột phải (hoặc bấm) biểu tượng ở khay → **Cài đặt…**:

- **Ngôn ngữ:** Theo Windows / Tiếng Việt / English.
- **Định dạng trên vòng tròn:** với mỗi loại file (Ảnh, GIF, Video, Âm thanh), chọn tối đa 8 định dạng hay dùng và kéo để sắp xếp thứ tự.
- Bật/tắt chuyển file gốc vào Thùng rác, và cử chỉ Shift khi kéo file.

## Định dạng hỗ trợ

In đậm là các định dạng có sẵn trên vòng tròn; các định dạng còn lại bật trong Cài đặt.

| Loại file | Đổi sang |
| --- | --- |
| **Ảnh** (jpg, png, webp, heic, avif, bmp, tiff, ico, jxl, exr, psd…) | **JPG, PNG, WEBP, AVIF, TIFF, BMP, ICO**, JPEG, GIF, JXL, JP2, TGA, QOI, EXR, HDR, PPM, PCX |
| **GIF** | **MP4, WEBM, MOV, WEBP động, PNG, JPG** (khung đầu), MKV, APNG |
| **Video** (mp4, mov, mkv, webm, avi, wmv, flv, ts…) | **MP4, MOV, MKV, WEBM, AVI, GIF, MP3, M4A**, HEVC/H.265, WMV, FLV, TS, MPG, OGV, ProRes, WAV, FLAC, OPUS |
| **Âm thanh** (mp3, m4a, wav, flac, ogg, opus, aiff, wma…) | **MP3, M4A, WAV, FLAC, OGG, OPUS, AIFF, WMA**, AAC, AC3, ALAC, M4R (nhạc chuông iPhone), MKA |

## Gỡ cài đặt

**Settings → Apps → Installed apps → KT Convert → Uninstall**. Để thoát app mà không gỡ: chuột phải biểu tượng khay → *Thoát KT Convert*.

## Câu hỏi thường gặp

- **Nhấn Ctrl+Alt+C không thấy gì?** Hãy chọn file trong File Explorer hoặc trên desktop trước. Nếu app khác đã chiếm phím tắt này, KT Convert sẽ báo ngay khi khởi động.
- **Có gửi file lên mạng không?** Không. Mọi chuyển đổi chạy offline trên máy bằng FFmpeg đi kèm.
- **Vì sao Windows cảnh báo khi cài?** App chưa được ký số (code signing). Bộ cài chỉ chứa KT Convert và FFmpeg; mã nguồn có đầy đủ trong repo này.

---

## Dành cho lập trình viên

Viết bằng [Tauri 2](https://tauri.app) (Rust) + TypeScript, không dùng framework giao diện.

**Cần:** Rust (MSVC), Node 20+, Visual Studio Build Tools (C++) + Windows SDK, và `ffmpeg.exe` trên PATH.

```bash
npm install
npm run tauri dev
```

Xem riêng giao diện trên trình duyệt (chế độ demo, không cần Rust): `npm run dev` rồi mở `http://localhost:1420/wheel.html` hoặc `/settings.html`.

Test chuyển đổi mọi định dạng trên file mẫu trong `samples/`:

```bash
cd src-tauri
cargo test --lib -- --nocapture
```

**Build bộ cài:** đặt `ffmpeg.exe` vào `src-tauri/ffmpeg/` (không đưa lên git vì ~217 MB), rồi chạy `npm run tauri build`. Kết quả ở `src-tauri/target/release/bundle/nsis/`. Đặt biến môi trường `KTC_DEBUG=1` để in log của bộ phát hiện cử chỉ kéo.

### Cấu trúc

| File | Vai trò |
| --- | --- |
| `src-tauri/src/lib.rs` | Khởi động, khay hệ thống, phím tắt, lệnh cho giao diện |
| `src-tauri/src/explorer.rs` | Đọc file đang chọn trong Explorer/desktop qua Shell COM (hỗ trợ tab Windows 11) |
| `src-tauri/src/dragwatch.rs` | Phát hiện "Shift khi đang kéo file" |
| `src-tauri/src/formats.rs` | Danh mục định dạng, tham số FFmpeg |
| `src-tauri/src/convert.rs` | Hàng đợi chuyển đổi, gọi FFmpeg, tiến độ |
| `src-tauri/src/backdrop.rs` | Chụp nền phía sau cửa sổ cho hiệu ứng kính |
| `src-tauri/src/i18n.rs`, `src/i18n.ts` | Chuỗi tiếng Việt / tiếng Anh |
| `src/wheel.*`, `src/toast.*`, `src/settings.*` | Vòng tròn, thẻ thông báo, cửa sổ Cài đặt |
| `src/glass-core.ts`, `src/glass.worker.ts` | Dựng lớp kính trong Web Worker |

### Giấy phép bên thứ ba

Bộ cài kèm FFmpeg (bản full build của [gyan.dev](https://www.gyan.dev/ffmpeg/builds/)), giấy phép **GPL v3** — xem `src-tauri/ffmpeg/FFMPEG-LICENSE.txt`. Mã nguồn FFmpeg: <https://ffmpeg.org/download.html>.
