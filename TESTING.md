# Hướng dẫn tự test KT Convert

## 1. Chuẩn bị

1. Cài app bằng file `KT Convert_0.1.0_x64-setup.exe` (trong `src-tauri\target\release\bundle\nsis\`). Bộ cài mặc định cài cho riêng tài khoản của bạn, vào `%LOCALAPPDATA%\KT Convert\`.
2. App cần `ffmpeg.exe` và `ffprobe.exe`. Máy bạn đã có sẵn trên PATH (bản cài qua winget) nên không phải làm gì thêm. Trên máy khác: chép hai file đó vào thư mục `bin\` cạnh `ktconvert.exe`.
3. Mở app từ Start menu. App không có cửa sổ chính, chỉ có biểu tượng ở khay hệ thống (góc phải dưới, có thể nằm trong mũi tên ^). Lần đầu mở sẽ có một thẻ thông báo nhỏ ở góc phải dưới.
4. File mẫu để thử nằm trong `E:\KTConvertAIO\samples\` (`photo.png`, `logo-transparent.png`, `anim.gif`, `clip.mkv`, `tone.wav`). Nên chép chúng ra một thư mục riêng, vì file kết quả được lưu ngay cạnh file gốc.

## 2. Phím tắt Ctrl+Alt+C

| Bước | Kết quả mong đợi |
| --- | --- |
| Mở thư mục mẫu trong File Explorer, chọn `photo.png`, nhấn **Ctrl+Alt+C** | Vòng tròn kính mờ hiện ra, tiêu đề là `photo.png`, các ô JPG, WEBP, AVIF, TIFF, BMP, ICO (không có PNG vì file đã là PNG) |
| Bấm vào ô **JPG** | Vòng tròn đóng; thẻ thông báo góc phải dưới hiện tiến độ rồi báo xong; `photo.jpg` xuất hiện cạnh file gốc |
| Làm lại với JPG | Lần này ra `photo (1).jpg`, file cũ không bị ghi đè |
| Mở vòng tròn, thử phím **mũi tên + Enter**, phím số **1–9**, rồi **Esc** | Mũi tên/số chọn được ô; Esc đóng vòng tròn mà không chuyển đổi |
| Chọn cùng lúc `photo.png` và `logo-transparent.png` → JPG | Tiêu đề "2 files"; ra 2 file JPG. `logo-transparent.jpg` có nền **trắng** (không phải đen) |
| `anim.gif` → MP4, WEBP | Ra video MP4 chạy được, WEBP vẫn động |
| `clip.mkv` → GIF, MP3 | GIF động; MP3 có tiếng |
| Chọn file trên **Desktop** rồi nhấn Ctrl+Alt+C | Hoạt động giống như trong Explorer |
| Thử trong một **tab thứ hai** của Explorer (Windows 11) | Lấy đúng file đang chọn ở tab đang mở |
| Không chọn gì, nhấn Ctrl+Alt+C | Thông báo "No files selected" |
| Chọn file không hỗ trợ (ví dụ `.txt`) | Thông báo "Unsupported file type" |
| Chuột phải biểu tượng khay → "Convert selected files" | Mở vòng tròn cho file đang chọn ở cửa sổ Explorer gần nhất |

## 3. Cử chỉ: đang kéo file thì nhấn Shift

1. Trong Explorer, nhấn giữ chuột trái trên `photo.png` và kéo đi một đoạn (đừng thả).
2. Trong khi vẫn giữ chuột, nhấn **Shift**.
3. Mong đợi: vòng tròn hiện ngay dưới con trỏ. Kéo tiếp lên một ô định dạng rồi thả chuột → chuyển đổi. Thả vào giữa vòng tròn → huỷ.
4. Thử cả: kéo từ Desktop; kéo khung chọn (rubber-band) trên vùng trống rồi nhấn Shift (vòng tròn **không** được hiện); kéo thanh tiêu đề cửa sổ rồi nhấn Shift (**không** được hiện).
5. Có thể tắt cử chỉ này: chuột phải biểu tượng khay → bỏ tích "Shift while dragging opens the wheel".

Phần nhận biết "đang kéo file" hiện **mới là phỏng đoán**: app dựa vào tên lớp (class) của cửa sổ đang giữ chuột, mà các tên đó chưa được kiểm tra trên máy thật. Nếu nhấn Shift khi kéo mà vòng tròn không hiện (hoặc hiện sai lúc), làm theo mục 4 và gửi log cho mình.

## 4. Bật log chẩn đoán (KTC_DEBUG)

Bản release không có cửa sổ console, nên phải chuyển log vào file:

1. Thoát app đang chạy: chuột phải biểu tượng khay → **Quit KT Convert** (app chỉ cho chạy một bản).
2. Mở PowerShell và chạy:

```powershell
$env:KTC_DEBUG = "1"; Start-Process "$env:LOCALAPPDATA\KT Convert\ktconvert.exe" -RedirectStandardError "$env:TEMP\ktc-debug.log"
```

3. Thử lại cử chỉ ở mục 3: kéo file **từ Explorer**, **từ Desktop**, kéo **khung chọn**, kéo **thanh tiêu đề**; mỗi lần nhấn Shift khi đang kéo.
4. Xem log:

```powershell
Get-Content "$env:TEMP\ktc-debug.log"
```

Mỗi lần nhấn Shift khi đang kéo sẽ có một dòng dạng:

```
[dragwatch] shift+drag, capture class = "..."
```

**Gửi lại cho mình các dòng đó, ghi kèm lúc ấy bạn đang kéo gì** (file trong Explorer / file trên Desktop / khung chọn / thanh tiêu đề). Từ đó mình sẽ sửa danh sách tên lớp trong `is_ole_drag` (`src-tauri/src/dragwatch.rs`). Nếu dòng đó không xuất hiện lần nào, hãy báo cả điều đó.

## 5. Nếu có lỗi

- Thẻ thông báo đỏ "FFmpeg not found": kiểm tra lệnh `ffmpeg -version` trong PowerShell, hoặc chép `ffmpeg.exe` + `ffprobe.exe` vào `%LOCALAPPDATA%\KT Convert\bin\`.
- "Ctrl+Alt+C is taken": app khác đang dùng phím tắt này; báo cho mình biết là app nào.
- Vòng tròn hiện nhưng nền kính bị đen/trong suốt hoàn toàn: chụp màn hình gửi mình.
- Máy lag hoặc treo: dừng ngay và báo cho mình. Chuyển video dài sang GIF tốn nhiều RAM (xem phần ghi chú trong báo cáo).
