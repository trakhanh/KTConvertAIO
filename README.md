# KT Convert

Trình chuyển đổi file "zero-click" cho Windows, lấy ý tưởng từ Tangerine (macOS). Chạy nền ở khay hệ thống, xử lý offline 100% bằng FFmpeg.

**Tác giả:** Khánh Trà · [github.com/trakhanh/KTConvertAIO](https://github.com/trakhanh/KTConvertAIO)

## Cách dùng

| Cách mở vòng chọn định dạng | Thao tác |
| --- | --- |
| Phím tắt | Chọn file trong File Explorer hoặc trên desktop rồi nhấn **Ctrl+Alt+C**. Chọn định dạng bằng chuột, phím mũi tên + Enter, hoặc phím số 1–9. Esc để huỷ. |
| Cử chỉ kéo | Đang kéo file, nhấn **Shift**: vòng tròn hiện dưới con trỏ. Thả file lên ô định dạng muốn chuyển; thả vào giữa để huỷ. |
| Khay hệ thống | Menu "Convert selected files" dùng cửa sổ Explorer gần nhất. |

File mới được lưu cạnh file gốc (`anh.png` → `anh.jpg`, trùng tên thì thành `anh (1).jpg`). File gốc giữ nguyên.

## Định dạng hỗ trợ

Vòng tròn hiện tối đa 8 định dạng cho mỗi loại file. Chọn và sắp xếp chúng trong **Settings…** (menu khay hệ thống); chưa chọn thì dùng bộ mặc định (in đậm).

- **Ảnh** (jpg, png, webp, heic, avif, bmp, tiff, ico, jxl, exr, …) → **JPG, PNG, WEBP, AVIF, TIFF, BMP, ICO**, GIF, JXL, JP2, TGA, QOI, EXR, HDR, PPM, PCX
- **GIF** → **MP4, WEBM, MOV, WEBP động, PNG/JPG (khung đầu)**, MKV, APNG
- **Video** (mp4, mov, mkv, webm, avi, wmv, …) → **MP4, MOV, MKV, WEBM, AVI, GIF, MP3, M4A**, HEVC (H.265, .mp4), WMV, FLV, TS, MPG, OGV, ProRes (.mov), WAV, FLAC, OPUS
- **Âm thanh** (mp3, m4a, wav, flac, ogg, opus, aiff, wma, …) → **MP3, M4A, WAV, FLAC, OGG, OPUS, AIFF, WMA**, AAC, AC3, ALAC (.m4a), M4R (nhạc chuông iPhone), MKA

Tuỳ chọn trong Settings: chuyển file gốc vào Thùng rác sau khi chuyển đổi (cũng bật/tắt bằng nút giữa vòng tròn hoặc phím D), và bật/tắt cử chỉ Shift khi kéo.

## Phát triển

Cần: Rust (MSVC), Node 20+, VC++ Build Tools + Windows SDK, và `ffmpeg.exe`/`ffprobe.exe` trên PATH (hoặc đặt trong thư mục `bin\` cạnh file exe).

```bash
npm install
npm run tauri dev
```

Xem riêng giao diện vòng tròn trên trình duyệt (chế độ demo, không cần Rust):

```bash
npm run dev
```

rồi mở `http://localhost:1420/wheel.html`.

Đặt biến môi trường `KTC_DEBUG=1` để in log của bộ phát hiện cử chỉ kéo.

## Cấu trúc

- `src-tauri/src/lib.rs`: khởi động, khay hệ thống, phím tắt, các lệnh gọi từ giao diện
- `src-tauri/src/explorer.rs`: đọc file đang chọn trong Explorer/desktop qua Shell COM (có hỗ trợ tab Windows 11)
- `src-tauri/src/dragwatch.rs`: phát hiện "Shift khi đang kéo file"
- `src-tauri/src/formats.rs`: phân loại file, danh sách định dạng đích, tham số FFmpeg
- `src-tauri/src/convert.rs`: hàng đợi chuyển đổi, gọi FFmpeg, báo tiến độ
- `src-tauri/src/toast.rs`: thẻ thông báo góc phải dưới
- `src/wheel.ts`, `src/wheel.css`: vòng tròn chọn định dạng
- `src/toast.ts`, `src/toast.css`: thẻ thông báo

## Ngôn ngữ

Giao diện có tiếng Việt và tiếng Anh: Settings → Ngôn ngữ (mặc định theo ngôn ngữ hiển thị của Windows). Chuỗi phía Rust nằm trong `src-tauri/src/i18n.rs`, phía giao diện trong `src/i18n.ts`; gợi ý của từng định dạng có bản tiếng Việt trong `formats.rs` (`hint_vi`).

## Bộ cài

```bash
npm run tauri build
```

Ra file `src-tauri/target/release/bundle/nsis/KT Convert_<version>_x64-setup.exe` (~57 MB). Trình cài đặt có tiếng Việt và tiếng Anh, cài cho tài khoản hiện tại (không cần quyền admin), kèm sẵn `bin/ffmpeg.exe`. Trước khi build, đặt `ffmpeg.exe` vào `src-tauri/ffmpeg/` (thư mục này không đưa lên git vì file ~217 MB). App không cần `ffprobe`: thông tin file được đọc từ `ffmpeg -i`.

## Ghi chú phát hành

Bản FFmpeg "full build" là giấy phép GPL v3 (bộ cài kèm `FFMPEG-LICENSE.txt`). Khi phát hành thương mại nên dùng bản FFmpeg LGPL tự build (vẫn có libwebp, libvpx, libopus, libaom, libmp3lame, nhưng không có libx264; khi đó dùng `h264_mf` của Windows để mã hoá H.264).
