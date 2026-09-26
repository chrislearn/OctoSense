//! 头像工坊 —— 纯本地头像图像处理模块。
//!
//! 与 UI 解耦:UI(由集成方接入)只需提供文件路径或字节,载入后得到
//! [`AvatarEditSession`],依次做 EXIF 朝向校正、裁切、旋转(90° 步进)、
//! 缩放,最后 [`AvatarEditSession::encode_final`] 产出可直接
//! `POST /api/v1/me/avatar` 的字节 + Content-Type。
//!
//! 解码/编码走 `image` crate(png / jpeg / webp);EXIF orientation 由本模块
//! 手写解析(JPEG APP1 与 WebP EXIF chunk 里的 TIFF 结构),无额外依赖。
//! 所有错误以 [`AvatarError`] 返回,不 panic;日志与错误信息不含文件内容,
//! 只含尺寸等无量化元数据。

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::{
    guess_format, load_from_memory_with_format, DynamicImage, GenericImageView, ImageEncoder,
    ImageFormat, RgbaImage,
};
use std::fmt;
use std::path::Path;

/// 输出边长上限(像素)。服务端与 UI 统一按 512×512 约束。
pub const MAX_OUTPUT_DIM: u32 = 512;
/// 载入字节数上限,防超大文件占爆内存。
pub const MAX_INPUT_BYTES: u64 = 32 * 1024 * 1024;
/// 解码后尺寸上限(宽/高),防 decompression bomb。
pub const MAX_INPUT_DIM: u32 = 16_384;
/// 最终编码字节数上限,超过时自动降 JPEG 质量重试。
pub const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

/// 输出编码格式。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AvatarFormat {
    Jpeg,
    Png,
    WebP,
}

impl AvatarFormat {
    /// 对应的 HTTP Content-Type。
    pub fn content_type(self) -> &'static str {
        match self {
            AvatarFormat::Jpeg => "image/jpeg",
            AvatarFormat::Png => "image/png",
            AvatarFormat::WebP => "image/webp",
        }
    }

    /// 是否支持有损质量参数。
    fn is_lossy(self) -> bool {
        matches!(self, AvatarFormat::Jpeg | AvatarFormat::WebP)
    }
}

/// 模块统一错误。消息为静态描述,不携带路径/字节内容等敏感数据。
#[derive(Debug)]
pub enum AvatarError {
    /// 读文件失败(不存在、无权限等)。
    Io(std::io::Error),
    /// 文件超过 [`MAX_INPUT_BYTES`]。
    InputTooLarge,
    /// 解码后尺寸超过 [`MAX_INPUT_DIM`]。
    DimensionsTooLarge,
    /// 无法识别或不支持的格式(仅 JPEG/PNG/WebP)。
    UnsupportedFormat,
    /// 数据损坏或不是合法图像。
    CorruptImage,
    /// 裁切区域为空或越界。
    InvalidCrop,
    /// 编码失败。
    EncodeFailed,
    /// 降到允许质量后编码结果仍超过 [`MAX_OUTPUT_BYTES`]。
    OutputTooLarge,
}

impl fmt::Display for AvatarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AvatarError::Io(e) => write!(f, "读取文件失败: {e}"),
            AvatarError::InputTooLarge => write!(f, "图片文件过大"),
            AvatarError::DimensionsTooLarge => write!(f, "图片尺寸过大"),
            AvatarError::UnsupportedFormat => write!(f, "仅支持 JPEG / PNG / WebP"),
            AvatarError::CorruptImage => write!(f, "图片损坏或不是合法图像"),
            AvatarError::InvalidCrop => write!(f, "裁切区域无效"),
            AvatarError::EncodeFailed => write!(f, "图片编码失败"),
            AvatarError::OutputTooLarge => write!(f, "编码结果超过大小上限"),
        }
    }
}

impl std::error::Error for AvatarError {}

impl From<std::io::Error> for AvatarError {
    fn from(e: std::io::Error) -> Self {
        AvatarError::Io(e)
    }
}

/// 最终编码结果:字节 + Content-Type,直接供上传使用。
pub struct EncodedAvatar {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub width: u32,
    pub height: u32,
}

/// 头像编辑会话:载入原图 → 校正操作 → [`encode_final`](Self::encode_final)。
///
/// 载入时已完成 EXIF 朝向校正,之后所有操作都基于「视觉正向」像素,
/// 输出不再带 EXIF(避免二次旋转,也避免携带拍摄元数据)。
pub struct AvatarEditSession {
    image: RgbaImage,
}

impl AvatarEditSession {
    /// 从文件路径载入(接入点之一;文件选择 UI 由集成方实现)。
    pub fn load_path(path: &Path) -> Result<Self, AvatarError> {
        let meta = std::fs::metadata(path)?;
        if meta.len() > MAX_INPUT_BYTES {
            return Err(AvatarError::InputTooLarge);
        }
        let bytes = std::fs::read(path)?;
        Self::load_bytes(&bytes)
    }

    /// 从内存字节载入(接入点之二)。识别格式 → EXIF 朝向校正 → 尺寸上限检查。
    pub fn load_bytes(bytes: &[u8]) -> Result<Self, AvatarError> {
        if bytes.len() as u64 > MAX_INPUT_BYTES {
            return Err(AvatarError::InputTooLarge);
        }
        let format = guess_format(bytes).map_err(|_| AvatarError::UnsupportedFormat)?;
        match format {
            ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP => {}
            _ => return Err(AvatarError::UnsupportedFormat),
        }
        let orientation = exif_orientation(bytes, format).unwrap_or(1);
        let image = load_from_memory_with_format(bytes, format)
            .map_err(|_| AvatarError::CorruptImage)?;
        let (w, h) = image.dimensions();
        if w > MAX_INPUT_DIM || h > MAX_INPUT_DIM {
            return Err(AvatarError::DimensionsTooLarge);
        }
        let corrected = apply_orientation(image, orientation);
        Ok(Self {
            image: corrected.to_rgba8(),
        })
    }

    /// 当前像素尺寸(宽, 高)。
    pub fn dimensions(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    /// 裁切指定区域(原图坐标,像素)。越界或空区域报 [`AvatarError::InvalidCrop`]。
    pub fn crop(mut self, x: u32, y: u32, width: u32, height: u32) -> Result<Self, AvatarError> {
        let (w, h) = self.image.dimensions();
        if width == 0 || height == 0 || x >= w || y >= h || x + width > w || y + height > h {
            return Err(AvatarError::InvalidCrop);
        }
        self.image = image::imageops::crop_imm(&self.image, x, y, width, height).to_image();
        Ok(self)
    }

    /// 以最长边为 `width` 的正方形居中裁切(头像常用)。
    pub fn crop_square_center(self) -> Result<Self, AvatarError> {
        let (w, h) = self.image.dimensions();
        let side = w.min(h);
        if side == 0 {
            return Err(AvatarError::InvalidCrop);
        }
        let x = (w - side) / 2;
        let y = (h - side) / 2;
        self.crop(x, y, side, side)
    }

    /// 顺时针旋转 `quarter_turns` 个 90°(取模 4)。
    pub fn rotate_quarters(mut self, quarter_turns: u32) -> Self {
        self.image = match quarter_turns % 4 {
            0 => self.image,
            1 => image::imageops::rotate90(&self.image),
            2 => image::imageops::rotate180(&self.image),
            _ => image::imageops::rotate270(&self.image),
        };
        self
    }

    /// 限制最长边不超过 [`MAX_OUTPUT_DIM`],保持宽高比(Lanczos3)。
    pub fn fit_within_max(mut self) -> Self {
        self.fit_within(MAX_OUTPUT_DIM)
    }

    /// 限制最长边不超过 `max_dim`,保持宽高比。已小于等于则不变。
    pub fn fit_within(mut self, max_dim: u32) -> Self {
        let (w, h) = self.image.dimensions();
        let longest = w.max(h);
        if max_dim == 0 || longest <= max_dim {
            return self;
        }
        let scale = max_dim as f64 / longest as f64;
        let nw = ((w as f64) * scale).round().max(1.0) as u32;
        let nh = ((h as f64) * scale).round().max(1.0) as u32;
        self.image = image::imageops::resize(
            &self.image,
            nw,
            nh,
            image::imageops::FilterType::Lanczos3,
        );
        self
    }

    /// 输出最终字节。先按 `format` 编码;有损格式在超过
    /// [`MAX_OUTPUT_BYTES`] 时自动降质量重试。
    pub fn encode_final(&self, format: AvatarFormat) -> Result<EncodedAvatar, AvatarError> {
        let (w, h) = self.image.dimensions();
        let qualities: &[u8] = if format.is_lossy() { &[85, 70, 50] } else { &[0] };
        for &q in qualities {
            let bytes = self.encode_with(format, q)?;
            if bytes.len() <= MAX_OUTPUT_BYTES {
                return Ok(EncodedAvatar {
                    bytes,
                    content_type: format.content_type(),
                    width: w,
                    height: h,
                });
            }
        }
        Err(AvatarError::OutputTooLarge)
    }

    fn encode_with(&self, format: AvatarFormat, quality: u8) -> Result<Vec<u8>, AvatarError> {
        let (w, h) = self.image.dimensions();
        let mut out = Vec::new();
        match format {
            AvatarFormat::Jpeg => {
                // JPEG 无 alpha:先铺到白底上,避免半透明像素编码出黑块。
                let mut opaque = image::RgbImage::new(w, h);
                for (x, y, px) in self.image.enumerate_pixels() {
                    let a = px[3] as u32;
                    let blend = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
                    opaque.put_pixel(x, y, image::Rgb([blend(px[0]), blend(px[1]), blend(px[2])]));
                }
                JpegEncoder::new_with_quality(&mut out, quality)
                    .write_image(opaque.as_raw(), w, h, image::ExtendedColorType::Rgb8)
                    .map_err(|_| AvatarError::EncodeFailed)?;
            }
            AvatarFormat::Png => {
                PngEncoder::new(&mut out)
                    .write_image(self.image.as_raw(), w, h, image::ExtendedColorType::Rgba8)
                    .map_err(|_| AvatarError::EncodeFailed)?;
            }
            AvatarFormat::WebP => {
                WebPEncoder::new_quality(&mut out, quality)
                    .write_image(self.image.as_raw(), w, h, image::ExtendedColorType::Rgba8)
                    .map_err(|_| AvatarError::EncodeFailed)?;
            }
        }
        Ok(out)
    }
}

/// 应用 EXIF orientation(1–8),返回视觉正向图像。未知值按 1 处理。
fn apply_orientation(img: DynamicImage, orientation: u8) -> DynamicImage {
    use image::imageops::{flip_horizontal, flip_vertical, rotate90, rotate180, rotate270};
    match orientation {
        2 => DynamicImage::ImageRgba8(flip_horizontal(&img.to_rgba8())),
        3 => DynamicImage::ImageRgba8(rotate180(&img.to_rgba8())),
        4 => DynamicImage::ImageRgba8(flip_vertical(&img.to_rgba8())),
        // 5–8 含转置,转置后宽高互换。
        5 => DynamicImage::ImageRgba8(flip_horizontal(&rotate90(&img.to_rgba8()))),
        6 => DynamicImage::ImageRgba8(rotate90(&img.to_rgba8())),
        7 => DynamicImage::ImageRgba8(flip_vertical(&rotate90(&img.to_rgba8()))),
        8 => DynamicImage::ImageRgba8(rotate270(&img.to_rgba8())),
        _ => img,
    }
}

/// 从原始字节里提取 EXIF orientation(tag 0x0112)。仅解析 JPEG APP1 与
/// WebP `EXIF` chunk;PNG 无 EXIF 直接返回 None。解析失败一律 None(按 1 处理)。
fn exif_orientation(bytes: &[u8], format: ImageFormat) -> Option<u8> {
    let tiff = match format {
        ImageFormat::Jpeg => find_jpeg_exif_tiff(bytes)?,
        ImageFormat::WebP => find_webp_exif_tiff(bytes)?,
        _ => return None,
    };
    parse_tiff_orientation(tiff)
}

/// 在 JPEG SOI 之后的段序列里找 APP1 "Exif\0\0",返回其后的 TIFF 数据切片。
fn find_jpeg_exif_tiff(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut pos = 2usize;
    // 最多扫 32 个段,防御构造恶意的段链。
    for _ in 0..32 {
        if pos + 4 > bytes.len() {
            return None;
        }
        if bytes[pos] != 0xFF {
            return None; // 段标记损坏
        }
        let marker = bytes[pos + 1];
        // SOS / EOI 之后不再有 APP1。
        if marker == 0xDA || marker == 0xD9 {
            return None;
        }
        // 独立标记(无长度字段)。
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            pos += 2;
            continue;
        }
        let seg_len = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > bytes.len() {
            return None;
        }
        let payload = &bytes[pos + 4..pos + 2 + seg_len];
        if marker == 0xE1 && payload.len() >= 6 && &payload[..6] == b"Exif\0\0" {
            return Some(&payload[6..]);
        }
        pos += 2 + seg_len;
    }
    None
}

/// 在 WebP RIFF chunk 序列里找 `EXIF` chunk,返回其中的 TIFF 数据切片。
fn find_webp_exif_tiff(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return None;
    }
    let mut pos = 12usize;
    while pos + 8 <= bytes.len() {
        let fourcc = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]])
            as usize;
        let body = pos + 8;
        if body + size > bytes.len() {
            return None;
        }
        if fourcc == b"EXIF" {
            return Some(&bytes[body..body + size]);
        }
        // chunk 按偶数字节对齐。
        pos = body + size + (size & 1);
    }
    None
}

/// 解析 TIFF 头 + 首个 IFD,取 orientation。同时支持小端(II)与大端(MM)。
fn parse_tiff_orientation(tiff: &[u8]) -> Option<u8> {
    if tiff.len() < 8 {
        return None;
    }
    let little = match &tiff[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16_at = |off: usize| -> Option<u16> {
        let b = tiff.get(off..off + 2)?;
        Some(if little {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        })
    };
    let u32_at = |off: usize| -> Option<u32> {
        let b = tiff.get(off..off + 4)?;
        Some(if little {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        })
    };
    if u16_at(2)? != 42 {
        return None;
    }
    let ifd = u32_at(4)? as usize;
    let count = u16_at(ifd)? as usize;
    for i in 0..count {
        let entry = ifd + 2 + i * 12;
        let tag = u16_at(entry)?;
        if tag == 0x0112 {
            let value = u16_at(entry + 8)?;
            return if (1..=8).contains(&value) {
                Some(value as u8)
            } else {
                None
            };
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage};

    /// 生成纯色测试图的原始 RGBA 缓冲。
    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(rgba))
    }

    /// 把图像编码为 PNG 字节(测试内联码,不走被测代码路径)。
    fn png_bytes(img: &RgbaImage) -> Vec<u8> {
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                ExtendedColorType::Rgba8,
            )
            .expect("test png encode");
        out
    }

    /// 把图像编码为 JPEG 字节。
    fn jpeg_bytes(img: &RgbaImage) -> Vec<u8> {
        let mut rgb = image::RgbImage::new(img.width(), img.height());
        for (x, y, px) in img.enumerate_pixels() {
            rgb.put_pixel(x, y, image::Rgb([px[0], px[1], px[2]]));
        }
        let mut out = Vec::new();
        JpegEncoder::new_with_quality(&mut out, 90)
            .write_image(rgb.as_raw(), img.width(), img.height(), ExtendedColorType::Rgb8)
            .expect("test jpeg encode");
        out
    }

    /// 生成一个小端 TIFF 块,首个 IFD 只含 orientation。
    fn tiff_with_orientation(orientation: u16) -> Vec<u8> {
        let mut t = Vec::new();
        t.extend_from_slice(b"II"); // little-endian
        t.extend_from_slice(&42u16.to_le_bytes());
        t.extend_from_slice(&8u32.to_le_bytes()); // IFD offset
        t.extend_from_slice(&1u16.to_le_bytes()); // 1 entry
        t.extend_from_slice(&0x0112u16.to_le_bytes()); // tag
        t.extend_from_slice(&3u16.to_le_bytes()); // type SHORT
        t.extend_from_slice(&1u32.to_le_bytes()); // count
        t.extend_from_slice(&orientation.to_le_bytes()); // value
        t.extend_from_slice(&[0, 0]); // padding
        t.extend_from_slice(&0u32.to_le_bytes()); // next IFD = 0
        t
    }

    /// 构造带 EXIF APP1 段的 JPEG:在 SOI 后插入 APP1。
    fn jpeg_with_exif(jpeg: &[u8], orientation: u16) -> Vec<u8> {
        let tiff = tiff_with_orientation(orientation);
        let mut app1_payload = b"Exif\0\0".to_vec();
        app1_payload.extend_from_slice(&tiff);
        let seg_len = (app1_payload.len() + 2) as u16;
        let mut out = Vec::new();
        out.extend_from_slice(&jpeg[..2]); // SOI
        out.push(0xFF);
        out.push(0xE1);
        out.extend_from_slice(&seg_len.to_be_bytes());
        out.extend_from_slice(&app1_payload);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    /// 构造带 EXIF chunk 的 WebP 文件字节。
    fn webp_with_exif(webp: &[u8], orientation: u16) -> Vec<u8> {
        let tiff = tiff_with_orientation(orientation);
        let riff_size = u32::from_le_bytes([webp[4], webp[5], webp[6], webp[7]]);
        let mut body = webp[8..].to_vec(); // "WEBP" + chunks
        body.extend_from_slice(b"EXIF");
        body.extend_from_slice(&(tiff.len() as u32).to_le_bytes());
        body.extend_from_slice(&tiff);
        if tiff.len() % 2 == 1 {
            body.push(0);
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(riff_size + 8 + tiff.len() as u32 + (tiff.len() as u32 & 1)).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn webp_bytes(img: &RgbaImage) -> Vec<u8> {
        let mut out = Vec::new();
        WebPEncoder::new_quality(&mut out, 90)
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                ExtendedColorType::Rgba8,
            )
            .expect("test webp encode");
        out
    }

    // ---------- 载入 / 错误处理 ----------

    #[test]
    fn loads_png_and_reports_dimensions() {
        let bytes = png_bytes(&solid(40, 20, [255, 0, 0, 255]));
        let session = AvatarEditSession::load_bytes(&bytes).expect("load png");
        assert_eq!(session.dimensions(), (40, 20));
    }

    #[test]
    fn loads_jpeg() {
        let bytes = jpeg_bytes(&solid(16, 16, [0, 255, 0, 255]));
        let session = AvatarEditSession::load_bytes(&bytes).expect("load jpeg");
        assert_eq!(session.dimensions(), (16, 16));
    }

    #[test]
    fn loads_webp() {
        let bytes = webp_bytes(&solid(12, 8, [0, 0, 255, 255]));
        let session = AvatarEditSession::load_bytes(&bytes).expect("load webp");
        // WebP 有损编码不改变尺寸。
        assert_eq!(session.dimensions(), (12, 8));
    }

    #[test]
    fn rejects_garbage_bytes() {
        let err = AvatarEditSession::load_bytes(b"not an image at all").unwrap_err();
        assert!(matches!(
            err,
            AvatarError::UnsupportedFormat | AvatarError::CorruptImage
        ));
    }

    #[test]
    fn rejects_truncated_png() {
        let bytes = png_bytes(&solid(10, 10, [1, 2, 3, 255]));
        let truncated = &bytes[..bytes.len() / 2];
        let err = AvatarEditSession::load_bytes(truncated).unwrap_err();
        assert!(matches!(err, AvatarError::CorruptImage));
    }

    #[test]
    fn rejects_unsupported_format_gif() {
        // 最小合法 GIF89a(1×1)。
        let gif: &[u8] = &[
            0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x21, 0xF9, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2C,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44, 0x01, 0x00,
            0x3B,
        ];
        let err = AvatarEditSession::load_bytes(gif).unwrap_err();
        assert!(matches!(err, AvatarError::UnsupportedFormat));
    }

    #[test]
    fn rejects_missing_file() {
        let err = AvatarEditSession::load_path(Path::new("/nonexistent/avatar.png")).unwrap_err();
        assert!(matches!(err, AvatarError::Io(_)));
    }

    // ---------- EXIF 朝向 ----------

    /// orientation 6(顺时针 90°):宽高的非方图载入后应宽高互换。
    #[test]
    fn exif_orientation_6_swaps_dimensions() {
        let img = solid(60, 20, [200, 100, 50, 255]);
        let jpeg = jpeg_bytes(&img);
        let with_exif = jpeg_with_exif(&jpeg, 6);
        let session = AvatarEditSession::load_bytes(&with_exif).expect("load exif jpeg");
        assert_eq!(session.dimensions(), (20, 60));
    }

    #[test]
    fn exif_orientation_3_rotates_180() {
        // 2×1 图:左红右蓝,orientation 3 后左蓝右红。
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, Rgba([0, 0, 255, 255]));
        let jpeg = jpeg_bytes(&img);
        let with_exif = jpeg_with_exif(&jpeg, 3);
        let session = AvatarEditSession::load_bytes(&with_exif).expect("load");
        assert_eq!(session.dimensions(), (2, 1));
        let left = session.image.get_pixel(0, 0);
        let right = session.image.get_pixel(1, 0);
        // JPEG 有损,允许色差。
        assert!(left[2] > 150 && left[0] < 100, "left should be blue: {left:?}");
        assert!(right[0] > 150 && right[2] < 100, "right should be red: {right:?}");
    }

    #[test]
    fn exif_orientation_2_flips_horizontal() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, Rgba([0, 0, 255, 255]));
        let jpeg = jpeg_bytes(&img);
        let with_exif = jpeg_with_exif(&jpeg, 2);
        let session = AvatarEditSession::load_bytes(&with_exif).expect("load");
        let left = session.image.get_pixel(0, 0);
        assert!(left[2] > 150, "left should be blue after hflip: {left:?}");
    }

    #[test]
    fn jpeg_without_exif_keeps_orientation() {
        let img = solid(30, 10, [1, 2, 3, 255]);
        let bytes = jpeg_bytes(&img);
        let session = AvatarEditSession::load_bytes(&bytes).expect("load");
        assert_eq!(session.dimensions(), (30, 10));
    }

    #[test]
    fn webp_exif_orientation_6_swaps_dimensions() {
        let img = solid(40, 12, [9, 9, 9, 255]);
        let webp = webp_bytes(&img);
        let with_exif = webp_with_exif(&webp, 6);
        let session = AvatarEditSession::load_bytes(&with_exif).expect("load webp exif");
        assert_eq!(session.dimensions(), (12, 40));
    }

    #[test]
    fn corrupt_exif_falls_back_to_orientation_1() {
        // APP1 里塞非 TIFF 数据,应按 1 处理且不报错。
        let img = solid(16, 6, [5, 5, 5, 255]);
        let jpeg = jpeg_bytes(&img);
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend_from_slice(b"garbage-not-tiff");
        let seg_len = (payload.len() + 2) as u16;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&jpeg[..2]);
        bytes.extend_from_slice(&[0xFF, 0xE1]);
        bytes.extend_from_slice(&seg_len.to_be_bytes());
        bytes.extend_from_slice(&payload);
        bytes.extend_from_slice(&jpeg[2..]);
        let session = AvatarEditSession::load_bytes(&bytes).expect("load");
        assert_eq!(session.dimensions(), (16, 6));
    }

    // ---------- 裁切 ----------

    #[test]
    fn crop_extracts_region() {
        let bytes = png_bytes(&solid(100, 80, [0, 0, 0, 255]));
        let session = AvatarEditSession::load_bytes(&bytes)
            .unwrap()
            .crop(10, 20, 30, 40)
            .expect("crop");
        assert_eq!(session.dimensions(), (30, 40));
    }

    #[test]
    fn crop_square_center_on_wide_image() {
        let bytes = png_bytes(&solid(100, 60, [0, 0, 0, 255]));
        let session = AvatarEditSession::load_bytes(&bytes)
            .unwrap()
            .crop_square_center()
            .expect("square");
        assert_eq!(session.dimensions(), (60, 60));
    }

    #[test]
    fn crop_rejects_out_of_bounds_and_empty() {
        let bytes = png_bytes(&solid(10, 10, [0, 0, 0, 255]));
        let s = || AvatarEditSession::load_bytes(&bytes).unwrap();
        assert!(matches!(s().crop(9, 0, 2, 2), Err(AvatarError::InvalidCrop)));
        assert!(matches!(s().crop(0, 0, 0, 5), Err(AvatarError::InvalidCrop)));
        assert!(matches!(s().crop(10, 0, 1, 1), Err(AvatarError::InvalidCrop)));
    }

    // ---------- 旋转 ----------

    #[test]
    fn rotate_quarters_swaps_dimensions() {
        let bytes = png_bytes(&solid(40, 20, [0, 0, 0, 255]));
        let s = || AvatarEditSession::load_bytes(&bytes).unwrap();
        assert_eq!(s().rotate_quarters(1).dimensions(), (20, 40));
        assert_eq!(s().rotate_quarters(2).dimensions(), (40, 20));
        assert_eq!(s().rotate_quarters(3).dimensions(), (20, 40));
        assert_eq!(s().rotate_quarters(4).dimensions(), (40, 20)); // 取模
        assert_eq!(s().rotate_quarters(0).dimensions(), (40, 20));
    }

    #[test]
    fn rotate_quarters_moves_pixels_correctly() {
        // 2×1 左红右蓝,顺时针 90° 后变 1×2:上红下蓝。
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, Rgba([0, 0, 255, 255]));
        let bytes = png_bytes(&img);
        let session = AvatarEditSession::load_bytes(&bytes)
            .unwrap()
            .rotate_quarters(1);
        assert_eq!(session.dimensions(), (1, 2));
        assert_eq!(session.image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(session.image.get_pixel(0, 1).0, [0, 0, 255, 255]);
    }

    // ---------- 尺寸限制 ----------

    #[test]
    fn fit_within_max_scales_down() {
        let bytes = png_bytes(&solid(2048, 1024, [0, 0, 0, 255]));
        let session = AvatarEditSession::load_bytes(&bytes)
            .unwrap()
            .fit_within_max();
        assert_eq!(session.dimensions(), (512, 256));
    }

    #[test]
    fn fit_within_keeps_small_images() {
        let bytes = png_bytes(&solid(100, 50, [0, 0, 0, 255]));
        let session = AvatarEditSession::load_bytes(&bytes)
            .unwrap()
            .fit_within_max();
        assert_eq!(session.dimensions(), (100, 50));
    }

    // ---------- 编码 ----------

    #[test]
    fn encode_final_jpeg_has_content_type() {
        let bytes = png_bytes(&solid(64, 64, [255, 0, 0, 255]));
        let out = AvatarEditSession::load_bytes(&bytes)
            .unwrap()
            .encode_final(AvatarFormat::Jpeg)
            .expect("encode jpeg");
        assert_eq!(out.content_type, "image/jpeg");
        assert_eq!(&out.bytes[..3], &[0xFF, 0xD8, 0xFF]);
        assert_eq!((out.width, out.height), (64, 64));
    }

    #[test]
    fn encode_final_png_and_webp() {
        let bytes = png_bytes(&solid(32, 32, [0, 255, 0, 255]));
        let s = || AvatarEditSession::load_bytes(&bytes).unwrap();
        let png = s().encode_final(AvatarFormat::Png).expect("png");
        assert_eq!(png.content_type, "image/png");
        assert_eq!(&png.bytes[1..4], b"PNG");
        let webp = s().encode_final(AvatarFormat::WebP).expect("webp");
        assert_eq!(webp.content_type, "image/webp");
        assert_eq!(&webp.bytes[..4], b"RIFF");
        assert_eq!(&webp.bytes[8..12], b"WEBP");
    }

    #[test]
    fn encoded_output_roundtrips_through_load() {
        // 完整流程:载入 → EXIF 校正 → 裁方 → 缩放 → 编码 → 再载入验证。
        let img = solid(1024, 512, [10, 20, 30, 255]);
        let jpeg = jpeg_bytes(&img);
        let with_exif = jpeg_with_exif(&jpeg, 6);
        let out = AvatarEditSession::load_bytes(&with_exif)
            .unwrap()
            .crop_square_center()
            .unwrap()
            .fit_within_max()
            .encode_final(AvatarFormat::WebP)
            .expect("encode");
        let reloaded = AvatarEditSession::load_bytes(&out.bytes).expect("reload");
        assert_eq!(reloaded.dimensions(), (512, 512));
    }

    #[test]
    fn full_pipeline_error_messages_are_static_and_safe() {
        // 错误 Display 不携带输入内容。
        let err = AvatarEditSession::load_bytes(b"secret-bytes-here").unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("secret"));
    }
}
