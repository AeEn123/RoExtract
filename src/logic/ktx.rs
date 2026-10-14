// KTX / KTX2 texture parsing for previews.
// Only one mip level is pulled out: the smallest one big enough for the preview,
// so large textures never get fully decompressed just to draw a thumbnail.

const KTX1_IDENTIFIER: &[u8; 12] = b"\xABKTX 11\xBB\r\n\x1A\n";
const KTX2_IDENTIFIER: &[u8; 12] = b"\xABKTX 20\xBB\r\n\x1A\n";

const KTX2_SUPERCOMPRESSION_NONE: u32 = 0;
const KTX2_SUPERCOMPRESSION_ZSTD: u32 = 2;

/// Pixel formats we can preview. sRGB variants map to the same entry: egui treats
/// texels as gamma-space, so sampling sRGB data through a UNORM format is correct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    Etc2Rgb,
    Etc2Rgba1,
    Etc2Rgba8,
    Bc1Rgb,
    Bc1Rgba,
    Bc3,
    Bc7,
    Rgba8,
}

impl TextureFormat {
    fn from_vk_format(vk_format: u32) -> Option<Self> {
        match vk_format {
            37 | 43 => Some(Self::Rgba8),     // R8G8B8A8_UNORM / _SRGB
            131 | 132 => Some(Self::Bc1Rgb),  // BC1_RGB_UNORM / _SRGB
            133 | 134 => Some(Self::Bc1Rgba), // BC1_RGBA_UNORM / _SRGB
            137 | 138 => Some(Self::Bc3),     // BC3_UNORM / _SRGB
            145 | 146 => Some(Self::Bc7),     // BC7_UNORM / _SRGB
            147 | 148 => Some(Self::Etc2Rgb), // ETC2_R8G8B8_UNORM / _SRGB
            149 | 150 => Some(Self::Etc2Rgba1),
            151 | 152 => Some(Self::Etc2Rgba8),
            _ => None,
        }
    }

    fn from_gl_internal_format(internal_format: u32) -> Option<Self> {
        match internal_format {
            0x8058 | 0x8C43 => Some(Self::Rgba8),   // RGBA8 / SRGB8_ALPHA8
            0x83F0 | 0x8C4C => Some(Self::Bc1Rgb),  // S3TC DXT1 RGB
            0x83F1 | 0x8C4D => Some(Self::Bc1Rgba), // S3TC DXT1 RGBA
            0x83F3 | 0x8C4F => Some(Self::Bc3),     // S3TC DXT5
            0x8E8C | 0x8E8D => Some(Self::Bc7),     // BPTC
            0x9274 | 0x9275 => Some(Self::Etc2Rgb), // RGB8_ETC2
            0x9276 | 0x9277 => Some(Self::Etc2Rgba1), // RGB8_PUNCHTHROUGH_ALPHA1_ETC2
            0x9278 | 0x9279 => Some(Self::Etc2Rgba8), // RGBA8_ETC2_EAC
            _ => None,
        }
    }

    /// The (UNORM) GL internal format used to upload this format.
    pub fn gl_internal_format(self) -> u32 {
        match self {
            Self::Etc2Rgb => 0x9274,
            Self::Etc2Rgba1 => 0x9276,
            Self::Etc2Rgba8 => 0x9278,
            Self::Bc1Rgb => 0x83F0,
            Self::Bc1Rgba => 0x83F1,
            Self::Bc3 => 0x83F3,
            Self::Bc7 => 0x8E8C,
            Self::Rgba8 => 0x8058,
        }
    }

    pub fn is_compressed(self) -> bool {
        self != Self::Rgba8
    }

    /// Bytes per 4x4 block (or per pixel for uncompressed formats).
    fn block_bytes(self) -> usize {
        match self {
            Self::Etc2Rgb | Self::Etc2Rgba1 | Self::Bc1Rgb | Self::Bc1Rgba => 8,
            Self::Etc2Rgba8 | Self::Bc3 | Self::Bc7 => 16,
            Self::Rgba8 => 4,
        }
    }

    /// Size in bytes of one image of `width` x `height` in this format.
    pub fn image_size(self, width: u32, height: u32) -> usize {
        if self.is_compressed() {
            (width as usize).div_ceil(4) * (height as usize).div_ceil(4) * self.block_bytes()
        } else {
            width as usize * height as usize * self.block_bytes()
        }
    }
}

/// One mip level of a texture, still in its native (usually block-compressed) format.
#[derive(Debug, Clone)]
pub struct KtxImage {
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

fn read_u32(bytes: &[u8], offset: usize, little_endian: bool) -> Result<u32, String> {
    let slice: [u8; 4] = bytes
        .get(offset..offset + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or("KTX file is truncated")?;
    Ok(if little_endian {
        u32::from_le_bytes(slice)
    } else {
        u32::from_be_bytes(slice)
    })
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let slice: [u8; 8] = bytes
        .get(offset..offset + 8)
        .and_then(|s| s.try_into().ok())
        .ok_or("KTX file is truncated")?;
    Ok(u64::from_le_bytes(slice))
}

fn level_dimension(base: u32, level: usize) -> u32 {
    base.checked_shr(level as u32).unwrap_or(0).max(1)
}

/// Pick the smallest mip level whose longest edge still covers `max_dimension`.
/// Falls back to level 0 when the texture is smaller than that.
pub fn choose_level(width: u32, height: u32, level_count: usize, max_dimension: u32) -> usize {
    (0..level_count.max(1))
        .rev()
        .find(|&level| {
            level_dimension(width, level).max(level_dimension(height, level)) >= max_dimension
        })
        .unwrap_or(0)
}

/// Extract the preview-sized mip level from a KTX1 or KTX2 file.
pub fn read_preview_level(bytes: &[u8], max_dimension: u32) -> Result<KtxImage, String> {
    match bytes.get(..12) {
        Some(id) if id == KTX2_IDENTIFIER => read_ktx2_level(bytes, max_dimension),
        Some(id) if id == KTX1_IDENTIFIER => read_ktx1_level(bytes, max_dimension),
        _ => Err("Not a KTX file".to_owned()),
    }
}

fn read_ktx2_level(bytes: &[u8], max_dimension: u32) -> Result<KtxImage, String> {
    let vk_format = read_u32(bytes, 12, true)?;
    let width = read_u32(bytes, 20, true)?;
    let height = read_u32(bytes, 24, true)?.max(1);
    let depth = read_u32(bytes, 28, true)?;
    let level_count = read_u32(bytes, 40, true)?.max(1) as usize;
    let supercompression = read_u32(bytes, 44, true)?;

    let format = TextureFormat::from_vk_format(vk_format)
        .ok_or_else(|| format!("Unsupported KTX2 vkFormat {}", vk_format))?;
    if width == 0 || depth > 1 {
        return Err("3D and 1D KTX textures are not supported".to_owned());
    }
    if level_count > 32 {
        return Err("KTX2 level count is invalid".to_owned());
    }

    let level = choose_level(width, height, level_count, max_dimension);
    // Level index starts after the 80 byte header; 24 bytes per level.
    let index = 80 + level * 24;
    let offset = read_u64(bytes, index)? as usize;
    let length = read_u64(bytes, index + 8)? as usize;
    let level_bytes = bytes
        .get(offset..offset.saturating_add(length))
        .ok_or("KTX2 level data is out of bounds")?;

    let (level_width, level_height) = (
        level_dimension(width, level),
        level_dimension(height, level),
    );
    let image_size = format.image_size(level_width, level_height);

    let mut data = match supercompression {
        KTX2_SUPERCOMPRESSION_NONE => level_bytes.to_vec(),
        KTX2_SUPERCOMPRESSION_ZSTD => {
            let uncompressed_length = read_u64(bytes, index + 16)? as usize;
            zstd::bulk::decompress(level_bytes, uncompressed_length.max(image_size))
                .map_err(|e| format!("Failed to decompress KTX2 level: {}", e))?
        }
        other => {
            return Err(format!(
                "Unsupported KTX2 supercompression scheme {}",
                other
            ))
        }
    };

    // A level holds every layer/face; keep only the first image.
    if data.len() < image_size {
        return Err("KTX2 level data is too short".to_owned());
    }
    data.truncate(image_size);

    Ok(KtxImage {
        format,
        width: level_width,
        height: level_height,
        data,
    })
}

fn read_ktx1_level(bytes: &[u8], max_dimension: u32) -> Result<KtxImage, String> {
    let little_endian = match bytes.get(12..16) {
        Some([0x01, 0x02, 0x03, 0x04]) => true,
        Some([0x04, 0x03, 0x02, 0x01]) => false,
        _ => return Err("KTX endianness marker is invalid".to_owned()),
    };
    let field = |offset| read_u32(bytes, offset, little_endian);

    let internal_format = field(28)?;
    let width = field(36)?;
    let height = field(40)?.max(1);
    let depth = field(44)?;
    let array_elements = field(48)?;
    let faces = field(52)?;
    let level_count = field(56)?.max(1) as usize;
    let key_value_bytes = field(60)? as usize;

    let format = TextureFormat::from_gl_internal_format(internal_format)
        .ok_or_else(|| format!("Unsupported KTX glInternalFormat {:#x}", internal_format))?;
    if width == 0 || depth > 1 {
        return Err("3D and 1D KTX textures are not supported".to_owned());
    }

    let level = choose_level(width, height, level_count, max_dimension);
    let mut offset = 64usize.saturating_add(key_value_bytes);

    for current in 0..=level {
        let image_size = field(offset)? as usize;
        offset += 4;
        if current == level {
            let (level_width, level_height) = (
                level_dimension(width, level),
                level_dimension(height, level),
            );
            let needed = format.image_size(level_width, level_height);
            let data = bytes
                .get(offset..offset.saturating_add(needed))
                .ok_or("KTX level data is out of bounds")?;
            if image_size < needed {
                return Err("KTX level data is too short".to_owned());
            }
            return Ok(KtxImage {
                format,
                width: level_width,
                height: level_height,
                data: data.to_vec(),
            });
        }
        // Non-array cubemaps store imageSize per face, each padded to 4 bytes.
        let level_size = if faces == 6 && array_elements == 0 {
            image_size.next_multiple_of(4) * 6
        } else {
            image_size
        };
        offset = offset.saturating_add(level_size.next_multiple_of(4));
    }
    unreachable!("loop always returns at the chosen level")
}

/// Decode on the CPU to straight-alpha RGBA8. Used when the GPU can't sample the format.
pub fn decode_to_rgba(image: &KtxImage) -> Result<Vec<u8>, String> {
    let (width, height) = (image.width as usize, image.height as usize);
    if image.format == TextureFormat::Rgba8 {
        return Ok(image.data.clone());
    }

    let mut pixels = vec![0u32; width * height];
    let decode = match image.format {
        TextureFormat::Etc2Rgb => texture2ddecoder::decode_etc2_rgb,
        TextureFormat::Etc2Rgba1 => texture2ddecoder::decode_etc2_rgba1,
        TextureFormat::Etc2Rgba8 => texture2ddecoder::decode_etc2_rgba8,
        TextureFormat::Bc1Rgb | TextureFormat::Bc1Rgba => texture2ddecoder::decode_bc1,
        TextureFormat::Bc3 => texture2ddecoder::decode_bc3,
        TextureFormat::Bc7 => texture2ddecoder::decode_bc7,
        TextureFormat::Rgba8 => unreachable!(),
    };
    decode(&image.data, width, height, &mut pixels)?;

    // texture2ddecoder packs pixels as BGRA in a little-endian u32.
    Ok(pixels
        .into_iter()
        .flat_map(|pixel| {
            let [b, g, r, a] = pixel.to_le_bytes();
            [r, g, b, a]
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a KTX2 file with one solid colour ETC2 RGB level per mip, zstd compressed.
    fn build_ktx2(width: u32, height: u32, levels: u32, zstd: bool) -> Vec<u8> {
        // Solid-ish ETC2 block: individual mode, base colours 0xF8,0x00,0x00 (red).
        let block: [u8; 8] = [0xF8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut level_data = Vec::new();
        for level in 0..levels as usize {
            let (w, h) = (
                level_dimension(width, level),
                level_dimension(height, level),
            );
            let raw: Vec<u8> =
                std::iter::repeat_n(block, TextureFormat::Etc2Rgb.image_size(w, h) / 8)
                    .flatten()
                    .collect();
            let stored = if zstd {
                zstd::bulk::compress(&raw, 3).unwrap()
            } else {
                raw.clone()
            };
            level_data.push((stored, raw.len()));
        }

        let mut out = Vec::new();
        out.extend_from_slice(KTX2_IDENTIFIER);
        for value in [
            147,
            1,
            width,
            height,
            0,
            0,
            1,
            levels,
            if zstd { 2 } else { 0 },
        ] {
            out.extend_from_slice(&u32::to_le_bytes(value));
        }
        out.extend_from_slice(&[0u8; 32]); // dfd/kvd/sgd offsets, unused here
        let mut offset = (80 + levels as usize * 24) as u64;
        for (stored, raw_len) in &level_data {
            out.extend_from_slice(&offset.to_le_bytes());
            out.extend_from_slice(&(stored.len() as u64).to_le_bytes());
            out.extend_from_slice(&(*raw_len as u64).to_le_bytes());
            offset += stored.len() as u64;
        }
        for (stored, _) in &level_data {
            out.extend_from_slice(stored);
        }
        out
    }

    #[test]
    fn test_choose_level_picks_smallest_covering_level() {
        assert_eq!(choose_level(1024, 1024, 11, 256), 2);
        assert_eq!(choose_level(1024, 512, 11, 256), 2);
        assert_eq!(choose_level(1024, 1024, 11, 300), 1);
        // Texture smaller than the preview: use the full size level.
        assert_eq!(choose_level(64, 64, 7, 256), 0);
        // Single level files always use level 0.
        assert_eq!(choose_level(1024, 1024, 1, 256), 0);
    }

    #[test]
    fn test_image_size_rounds_up_to_blocks() {
        assert_eq!(TextureFormat::Etc2Rgb.image_size(491, 449), 123 * 113 * 8);
        assert_eq!(TextureFormat::Etc2Rgba8.image_size(1, 1), 16);
        assert_eq!(TextureFormat::Rgba8.image_size(3, 2), 24);
    }

    #[test]
    fn test_ktx2_zstd_reads_preview_level() {
        let file = build_ktx2(1024, 512, 11, true);
        let image = read_preview_level(&file, 256).unwrap();
        assert_eq!(image.format, TextureFormat::Etc2Rgb);
        assert_eq!((image.width, image.height), (256, 128));
        assert_eq!(image.data.len(), 64 * 32 * 8);
    }

    #[test]
    fn test_ktx2_uncompressed_single_level() {
        let file = build_ktx2(491, 449, 1, false);
        let image = read_preview_level(&file, 256).unwrap();
        assert_eq!((image.width, image.height), (491, 449));
        assert_eq!(
            image.data.len(),
            TextureFormat::Etc2Rgb.image_size(491, 449)
        );
    }

    #[test]
    fn test_ktx2_decodes_to_rgba() {
        let file = build_ktx2(8, 8, 1, true);
        let image = read_preview_level(&file, 256).unwrap();
        let rgba = decode_to_rgba(&image).unwrap();
        assert_eq!(rgba.len(), 8 * 8 * 4);
        // Red-dominant, fully opaque.
        assert!(rgba[0] > rgba[1] && rgba[0] > rgba[2]);
        assert_eq!(rgba[3], 255);
    }

    #[test]
    fn test_ktx2_truncated_is_error_not_panic() {
        // Single level, so every truncation cuts into the level being read.
        let file = build_ktx2(64, 64, 1, true);
        for len in [0, 11, 40, 80, 100, file.len() - 1] {
            assert!(read_preview_level(&file[..len], 256).is_err());
        }
    }

    #[test]
    fn test_ktx1_reads_level() {
        let block = [0u8; 16];
        let mut file = Vec::new();
        file.extend_from_slice(KTX1_IDENTIFIER);
        // endianness, glType, glTypeSize, glFormat, glInternalFormat, glBaseInternalFormat,
        // width, height, depth, arrayElements, faces, mipLevels, keyValueBytes
        for value in [0x04030201u32, 0, 1, 0, 0x9278, 0x1908, 8, 4, 0, 0, 1, 2, 0] {
            file.extend_from_slice(&value.to_le_bytes());
        }
        file.extend_from_slice(&32u32.to_le_bytes()); // level 0: 2x1 blocks
        file.extend(std::iter::repeat_n(block, 2).flatten());
        file.extend_from_slice(&16u32.to_le_bytes()); // level 1: 1 block
        file.extend_from_slice(&[7u8; 16]);

        let image = read_preview_level(&file, 4).unwrap();
        assert_eq!(image.format, TextureFormat::Etc2Rgba8);
        assert_eq!((image.width, image.height), (4, 2));
        assert_eq!(image.data, vec![7u8; 16]);
    }
}
