//! Converting assets to other formats: images (including KTX textures) to
//! common image formats, and audio to WAV.

use std::io::Cursor;

use rodio::{Decoder, Source};

use crate::{
    config,
    logic::{ktx, AssetInfo, Category},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Png,
    Jpeg,
    Webp,
    Bmp,
    Wav,
}

impl Target {
    pub const IMAGE: [Target; 4] = [Target::Png, Target::Jpeg, Target::Webp, Target::Bmp];
    pub const AUDIO: [Target; 1] = [Target::Wav];

    pub fn extension(self) -> &'static str {
        match self {
            Target::Png => "png",
            Target::Jpeg => "jpg",
            Target::Webp => "webp",
            Target::Bmp => "bmp",
            Target::Wav => "wav",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Target::Png => "PNG",
            Target::Jpeg => "JPEG",
            Target::Webp => "WEBP",
            Target::Bmp => "BMP",
            Target::Wav => "WAV",
        }
    }

    pub fn from_extension(extension: &str) -> Option<Target> {
        Target::IMAGE
            .into_iter()
            .chain(Target::AUDIO)
            .find(|target| target.extension() == extension)
    }
}

fn is_image(asset: &AssetInfo) -> bool {
    matches!(asset.category, Category::Images | Category::Ktx)
        || matches!(asset.format, Some("png" | "webp" | "ktx"))
}

fn is_audio(asset: &AssetInfo) -> bool {
    matches!(asset.category, Category::Music | Category::Sounds)
        || matches!(asset.format, Some("ogg" | "mp3"))
}

/// The formats `asset` can be converted to.
pub fn targets_for(asset: &AssetInfo) -> &'static [Target] {
    if is_image(asset) {
        &Target::IMAGE
    } else if is_audio(asset) {
        &Target::AUDIO
    } else {
        &[]
    }
}

/// True when the user turned conversion on in the settings.
pub fn is_enabled() -> bool {
    config::get_config_bool("conversion_enabled").unwrap_or(false)
}

/// The format to convert `asset` to when extracting, if the user asked for that.
pub fn target_on_extract(asset: &AssetInfo) -> Option<Target> {
    if !is_enabled() || !config::get_config_bool("convert_on_extract").unwrap_or(true) {
        return None;
    }
    let (key, default) = if is_image(asset) {
        ("convert_image_format", Target::Png)
    } else if is_audio(asset) {
        ("convert_audio_format", Target::Wav)
    } else {
        return None;
    };
    let target = config::get_config_string(key)
        .and_then(|extension| Target::from_extension(&extension))
        .unwrap_or(default);
    targets_for(asset).contains(&target).then_some(target)
}

/// Convert the extracted bytes of `asset` to `target`.
pub fn convert(asset: &AssetInfo, bytes: &[u8], target: Target) -> Result<Vec<u8>, String> {
    if !targets_for(asset).contains(&target) {
        return Err(format!("Can't convert this asset to {}", target.name()));
    }
    match target {
        Target::Wav => audio_to_wav(bytes),
        _ => {
            let image = if asset.format == Some("ktx") || asset.category == Category::Ktx {
                ktx_to_image(bytes)?
            } else {
                image::load_from_memory(bytes).map_err(|e| e.to_string())?
            };
            encode_image(image, target)
        }
    }
}

fn ktx_to_image(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    let level = ktx::read_preview_level(bytes, u32::MAX)?; // Full size
    let rgba = ktx::decode_to_rgba(&level)?;
    image::RgbaImage::from_raw(level.width, level.height, rgba)
        .map(image::DynamicImage::ImageRgba8)
        .ok_or_else(|| "Decoded KTX image has the wrong size".to_owned())
}

fn encode_image(image: image::DynamicImage, target: Target) -> Result<Vec<u8>, String> {
    let (image, format) = match target {
        // JPEG has no alpha channel
        Target::Jpeg => (
            image::DynamicImage::ImageRgb8(image.to_rgb8()),
            image::ImageFormat::Jpeg,
        ),
        Target::Webp => (
            image::DynamicImage::ImageRgba8(image.to_rgba8()),
            image::ImageFormat::WebP,
        ),
        Target::Bmp => (
            image::DynamicImage::ImageRgba8(image.to_rgba8()),
            image::ImageFormat::Bmp,
        ),
        _ => (image, image::ImageFormat::Png),
    };
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, format)
        .map_err(|e| e.to_string())?;
    Ok(output.into_inner())
}

/// Decode audio and write it as 16-bit PCM WAV.
fn audio_to_wav(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let decoder = Decoder::builder()
        .with_data(Cursor::new(bytes.to_vec()))
        .with_byte_len(bytes.len() as u64)
        .with_seekable(true)
        .build()
        .map_err(|e| e.to_string())?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples: Vec<i16> = decoder
        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();
    Ok(wav_bytes(channels, sample_rate, &samples))
}

fn wav_bytes(channels: u16, sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let block_align = channels * 2;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * block_align as u32).to_le_bytes()); // Byte rate
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(category: Category, format: Option<&'static str>) -> AssetInfo {
        AssetInfo {
            name: "test".to_owned(),
            size: 0,
            last_modified: None,
            created: None,
            format,
            from_file: true,
            from_sql: false,
            from_rbx_storage: false,
            category,
        }
    }

    #[test]
    fn test_targets_for() {
        assert_eq!(
            targets_for(&asset(Category::Images, Some("png"))),
            &Target::IMAGE
        );
        assert_eq!(
            targets_for(&asset(Category::Ktx, Some("ktx"))),
            &Target::IMAGE
        );
        assert_eq!(
            targets_for(&asset(Category::Sounds, Some("ogg"))),
            &Target::AUDIO
        );
        assert!(targets_for(&asset(Category::Rbxm, Some("rbxm"))).is_empty());
    }

    #[test]
    fn test_wav_header() {
        let wav = wav_bytes(2, 44100, &[0, 1, -1, 2]);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(wav[4..8].try_into().unwrap()), 36 + 8);
        assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 2); // Channels
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 44100);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 8); // Data length
        assert_eq!(wav.len(), 44 + 8);
    }

    #[test]
    fn test_convert_png_to_each_image_format() {
        let mut png = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(4, 3, image::Rgba([255, 0, 0, 128]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let source = asset(Category::Images, Some("png"));

        for target in Target::IMAGE {
            let converted = convert(&source, &png, target).unwrap();
            let decoded = image::load_from_memory(&converted).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (4, 3), "{target:?}");
        }
        assert!(convert(&source, &png, Target::Wav).is_err());
    }

    #[test]
    fn test_convert_ktx_to_png() {
        // Uncompressed RGBA8 KTX1, 2x2
        let mut ktx = b"\xABKTX 11\xBB\r\n\x1A\n".to_vec();
        for value in [
            0x04030201u32,
            0x1401,
            1,
            0x1908,
            0x8058,
            0x1908,
            2,
            2,
            0,
            0,
            1,
            1,
            0,
        ] {
            ktx.extend_from_slice(&value.to_le_bytes());
        }
        ktx.extend_from_slice(&16u32.to_le_bytes());
        ktx.extend_from_slice(&[0, 255, 0, 255].repeat(4));

        let png = convert(&asset(Category::Ktx, Some("ktx")), &ktx, Target::Png).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (2, 2));
        assert_eq!(decoded.get_pixel(1, 1).0, [0, 255, 0, 255]);
    }
}
