//! Picture conversion with the `image` crate. Runs on a blocking thread.

use crate::error::ConvertError;
use crate::model::{Options, Target};
use image::codecs::{
    bmp::BmpEncoder, gif::GifEncoder, ico::IcoEncoder, jpeg::JpegEncoder, png::PngEncoder,
    tiff::TiffEncoder, webp::WebPEncoder,
};
use image::imageops::FilterType;
use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageFormat, ImageReader,
    RgbaImage,
};
use img_parts::{Bytes, ImageEXIF, ImageICC};
use std::io::Cursor;
use std::path::Path;

/// Pictures larger than this many pixels are refused before decoding (decompression bombs).
const MAX_PIXELS: u64 = 400_000_000;

pub struct Decoded {
    pub image: DynamicImage,
    pub format: Option<ImageFormat>,
    pub exif: Option<Bytes>,
    pub icc: Option<Bytes>,
    pub had_alpha: bool,
}

fn carries_metadata(format: Option<ImageFormat>) -> bool {
    matches!(
        format,
        Some(ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP)
    )
}

fn target_carries_metadata(target: Target) -> bool {
    matches!(target, Target::Jpg | Target::Png | Target::Webp)
}

fn read_meta(bytes: &[u8], format: Option<ImageFormat>) -> (Option<Bytes>, Option<Bytes>) {
    let data = Bytes::copy_from_slice(bytes);
    match format {
        Some(ImageFormat::Jpeg) => img_parts::jpeg::Jpeg::from_bytes(data)
            .map(|j| (j.exif(), j.icc_profile()))
            .unwrap_or((None, None)),
        Some(ImageFormat::Png) => img_parts::png::Png::from_bytes(data)
            .map(|p| (p.exif(), p.icc_profile()))
            .unwrap_or((None, None)),
        Some(ImageFormat::WebP) => img_parts::webp::WebP::from_bytes(data)
            .map(|w| (w.exif(), w.icc_profile()))
            .unwrap_or((None, None)),
        _ => (None, None),
    }
}

fn write_meta(
    encoded: Vec<u8>,
    target: Target,
    exif: Option<Bytes>,
    icc: Option<Bytes>,
) -> Result<Vec<u8>, ConvertError> {
    if exif.is_none() && icc.is_none() {
        return Ok(encoded);
    }
    let bad = |_| ConvertError::Other("couldn't copy the picture's metadata".into());
    let data = Bytes::from(encoded);
    let mut out = Vec::new();
    match target {
        Target::Jpg => {
            let mut j = img_parts::jpeg::Jpeg::from_bytes(data).map_err(bad)?;
            j.set_exif(exif);
            j.set_icc_profile(icc);
            j.encoder()
                .write_to(&mut out)
                .map_err(|e| ConvertError::Io(e.to_string()))?;
        }
        Target::Png => {
            let mut p = img_parts::png::Png::from_bytes(data).map_err(bad)?;
            p.set_exif(exif);
            p.set_icc_profile(icc);
            p.encoder()
                .write_to(&mut out)
                .map_err(|e| ConvertError::Io(e.to_string()))?;
        }
        Target::Webp => {
            let mut w = img_parts::webp::WebP::from_bytes(data).map_err(bad)?;
            w.set_exif(exif);
            w.set_icc_profile(icc);
            w.encoder()
                .write_to(&mut out)
                .map_err(|e| ConvertError::Io(e.to_string()))?;
        }
        _ => return Ok(data.to_vec()),
    }
    Ok(out)
}

/// Reads a picture. `rotate` applies the file's rotation flag to the pixels; leave it off only
/// when the flag is going to be copied into the result.
pub fn decode(path: &Path, rotate: bool) -> Result<Decoded, ConvertError> {
    let bytes = std::fs::read(path).map_err(|e| ConvertError::io(path, e))?;
    let reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|e| ConvertError::io(path, e))?;
    let format = reader.format();
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| ConvertError::NotReadable("a picture"))?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || (w as u64) * (h as u64) > MAX_PIXELS {
        return Err(ConvertError::Other(
            "This picture is too large to convert safely.".into(),
        ));
    }
    let orientation = decoder.orientation().ok();
    let icc = decoder.icc_profile().ok().flatten();
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|_| ConvertError::NotReadable("a picture"))?;
    let had_alpha = image.color().has_alpha();
    if let Some(o) = orientation {
        if rotate && o != image::metadata::Orientation::NoTransforms {
            image.apply_orientation(o);
        }
    }
    let (exif, icc2) = read_meta(&bytes, format);
    Ok(Decoded {
        image,
        format,
        exif,
        icc: icc2.or_else(|| icc.map(Bytes::from)),
        had_alpha,
    })
}

/// Fits inside the box without ever scaling up.
pub fn fit(image: DynamicImage, max_w: Option<u32>, max_h: Option<u32>) -> DynamicImage {
    let (w, h) = (image.width(), image.height());
    let bw = max_w.unwrap_or(u32::MAX).min(w.max(1));
    let bh = max_h.unwrap_or(u32::MAX).min(h.max(1));
    if bw >= w && bh >= h {
        return image;
    }
    image.resize(bw, bh, FilterType::Lanczos3)
}

/// Puts transparent areas on a solid colour and returns plain RGB.
pub fn flatten(image: &DynamicImage, bg: [u8; 3]) -> image::RgbImage {
    let rgba = image.to_rgba8();
    let mut out = image::RgbImage::new(rgba.width(), rgba.height());
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = p[3] as u32;
        let mix = |c: u8, b: u8| ((c as u32 * a + b as u32 * (255 - a) + 127) / 255) as u8;
        out.put_pixel(
            x,
            y,
            image::Rgb([mix(p[0], bg[0]), mix(p[1], bg[1]), mix(p[2], bg[2])]),
        );
    }
    out
}

fn eight_bit(image: &DynamicImage) -> RgbaImage {
    image.to_rgba8()
}

fn encode(image: &DynamicImage, target: Target, o: &Options) -> Result<Vec<u8>, ConvertError> {
    let enc =
        |e: image::ImageError| ConvertError::Other(format!("couldn't write the picture: {e}"));
    let mut buf: Vec<u8> = Vec::new();
    match target {
        Target::Jpg => {
            let rgb = flatten(image, o.background);
            let encoder = JpegEncoder::new_with_quality(&mut buf, o.image_quality);
            rgb.write_with_encoder(encoder).map_err(enc)?;
        }
        Target::Png => {
            let encoder = PngEncoder::new_with_quality(
                &mut buf,
                image::codecs::png::CompressionType::Default,
                image::codecs::png::FilterType::Adaptive,
            );
            // Keep 16-bit and grey pictures as they are; flatten anything else to 8-bit.
            let img = match image.color() {
                image::ColorType::L8
                | image::ColorType::La8
                | image::ColorType::Rgb8
                | image::ColorType::Rgba8
                | image::ColorType::L16
                | image::ColorType::La16
                | image::ColorType::Rgb16
                | image::ColorType::Rgba16 => image.clone(),
                _ => DynamicImage::ImageRgba8(eight_bit(image)),
            };
            img.write_with_encoder(encoder).map_err(enc)?;
        }
        Target::Webp => {
            let rgba = eight_bit(image);
            if o.image_quality >= 100 {
                let encoder = WebPEncoder::new_lossless(&mut buf);
                rgba.write_with_encoder(encoder).map_err(enc)?;
            } else {
                let (w, h) = rgba.dimensions();
                let mem = webp::Encoder::from_rgba(rgba.as_raw(), w, h)
                    .encode(f32::from(o.image_quality));
                buf = mem.to_vec();
            }
        }
        Target::Bmp => {
            let rgba = eight_bit(image);
            let mut encoder = BmpEncoder::new(&mut buf);
            encoder
                .encode(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(enc)?;
        }
        Target::Tiff => {
            let rgba = eight_bit(image);
            let mut cursor = Cursor::new(Vec::new());
            TiffEncoder::new(&mut cursor)
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(enc)?;
            buf = cursor.into_inner();
        }
        Target::Gif => {
            let rgba = eight_bit(image);
            {
                let mut encoder = GifEncoder::new_with_speed(&mut buf, 10);
                encoder.encode_frame(image::Frame::new(rgba)).map_err(enc)?;
            }
        }
        Target::Ico => {
            let rgba = eight_bit(image);
            IcoEncoder::new(&mut buf)
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(enc)?;
        }
        _ => return Err(ConvertError::Other("not a picture format".into())),
    }
    Ok(buf)
}

/// Converts `src` and writes the result to `tmp`. Returns notes to show the user.
pub fn convert(
    src: &Path,
    tmp: &Path,
    target: Target,
    o: &Options,
) -> Result<Vec<String>, ConvertError> {
    let mut notes = Vec::new();

    // Look at the source's format first to decide whether the rotation flag travels with it.
    let probe = std::fs::File::open(src)
        .ok()
        .and_then(|f| {
            ImageReader::new(std::io::BufReader::new(f))
                .with_guessed_format()
                .ok()
        })
        .and_then(|r| r.format());
    let copy_meta = o.keep_metadata && carries_metadata(probe) && target_carries_metadata(target);

    let d = decode(src, !copy_meta)?;
    let mut image = fit(d.image, o.max_width, o.max_height);

    if target == Target::Ico && (image.width() > 256 || image.height() > 256) {
        image = image.resize(256, 256, FilterType::Lanczos3);
        notes.push("Icons can be at most 256 × 256, so the picture was made smaller.".into());
    }
    if d.had_alpha && target == Target::Jpg {
        notes.push(
            "JPG has no transparency. Transparent areas were filled with a solid colour.".into(),
        );
    }
    if d.format == Some(ImageFormat::Gif) && target != Target::Gif {
        notes.push("Only the first frame of the GIF was used.".into());
    }
    if target == Target::Gif {
        notes.push("GIF has at most 256 colours, so the colours may look different.".into());
    }

    let mut bytes = encode(&image, target, o)?;
    if copy_meta {
        let exif = d.exif.clone();
        let icc = d.icc.clone();
        if exif.is_some() || icc.is_some() {
            bytes = write_meta(bytes, target, exif, icc)?;
        }
    } else if !o.keep_metadata && (d.exif.is_some()) {
        notes.push("Camera and location details were removed.".into());
    }
    std::fs::write(tmp, &bytes).map_err(|e| ConvertError::io(tmp, e))?;
    Ok(notes)
}

/// Checks that the written file opens as a picture of the expected kind.
pub fn validate(path: &Path, target: Target) -> Result<(), ConvertError> {
    let bytes = std::fs::read(path).map_err(|e| ConvertError::io(path, e))?;
    if bytes.is_empty() {
        return Err(ConvertError::BadOutput("The result is empty."));
    }
    let reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|e| ConvertError::io(path, e))?;
    let expected = match target {
        Target::Jpg => ImageFormat::Jpeg,
        Target::Png => ImageFormat::Png,
        Target::Webp => ImageFormat::WebP,
        Target::Bmp => ImageFormat::Bmp,
        Target::Tiff => ImageFormat::Tiff,
        Target::Gif => ImageFormat::Gif,
        Target::Ico => ImageFormat::Ico,
        _ => return Ok(()),
    };
    if reader.format() != Some(expected) {
        return Err(ConvertError::BadOutput(
            "The result isn't the format that was asked for.",
        ));
    }
    let img = reader
        .decode()
        .map_err(|_| ConvertError::BadOutput("The result can't be opened as a picture."))?;
    if img.width() == 0 || img.height() == 0 {
        return Err(ConvertError::BadOutput("The result has no picture in it."));
    }
    Ok(())
}
