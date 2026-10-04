//! 8-bit RGB pictures: PNG files in and out, and pixel-exact comparison.

use std::fs::File;
use std::io::{BufWriter, Cursor};
use std::path::Path;

use deadrally_core::expand_6bit;
use deadrally_gamedata::image::{Image, Palette};
use png::{BitDepth, ColorType, Transformations};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub width: u32,
    pub height: u32,
    /// Three bytes per pixel, row-major.
    pub pixels: Vec<u8>,
}

/// How far apart two pictures of the same size are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Difference {
    pub pixels: usize,
    pub max_channel: u8,
}

impl Rgb {
    /// An indexed image in its palette, expanded `v << 2` like the Windows version.
    pub fn from_indexed(image: &Image, palette: &Palette) -> Rgb {
        Rgb {
            width: image.width,
            height: image.height,
            pixels: image
                .pixels
                .iter()
                .flat_map(|&index| palette.0[usize::from(index)].map(expand_6bit))
                .collect(),
        }
    }

    /// Images side by side, left to right, on black; they share `palette`.
    pub fn strip(images: &[Image], palette: &Palette) -> Rgb {
        let width = images.iter().map(|image| image.width).sum();
        let height = images.iter().map(|image| image.height).max().unwrap_or(0);
        let mut strip = Rgb {
            width,
            height,
            pixels: vec![0; (width * height * 3) as usize],
        };
        let mut left = 0;
        for image in images {
            let rgb = Rgb::from_indexed(image, palette);
            let row_bytes = (image.width * 3) as usize;
            for (y, row) in rgb.pixels.chunks_exact(row_bytes).enumerate() {
                let start = (y * width as usize + left) * 3;
                strip.pixels[start..start + row_bytes].copy_from_slice(row);
            }
            left += image.width as usize;
        }
        strip
    }

    /// Reads any 8-bit or 16-bit PNG and converts it to RGB; alpha is dropped.
    pub fn read_png(path: &Path) -> Result<Rgb, String> {
        let fail = |error: &dyn std::fmt::Display| format!("{}: {error}", path.display());
        let bytes = std::fs::read(path).map_err(|error| fail(&error))?;
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|error| fail(&error))?;
        let size = reader
            .output_buffer_size()
            .ok_or_else(|| fail(&"too large"))?;
        let mut buffer = vec![0; size];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|error| fail(&error))?;
        buffer.truncate(info.buffer_size());
        let pixels = match info.color_type {
            ColorType::Rgb => buffer,
            ColorType::Rgba => buffer
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|&[r, g, b, _]| [r, g, b])
                .collect(),
            ColorType::Grayscale => buffer.iter().flat_map(|&grey| [grey; 3]).collect(),
            ColorType::GrayscaleAlpha => buffer
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|&[grey, _]| [grey; 3])
                .collect(),
            ColorType::Indexed => return Err(fail(&"palette PNG was not expanded")),
        };
        Ok(Rgb {
            width: info.width,
            height: info.height,
            pixels,
        })
    }

    pub fn write_png(&self, path: &Path) -> Result<(), String> {
        let fail = |error: &dyn std::fmt::Display| format!("{}: {error}", path.display());
        let file = File::create(path).map_err(|error| fail(&error))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), self.width, self.height);
        encoder.set_color(ColorType::Rgb);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|error| fail(&error))?;
        writer
            .write_image_data(&self.pixels)
            .map_err(|error| fail(&error))?;
        writer.finish().map_err(|error| fail(&error))
    }

    /// Pixel-by-pixel difference; `None` when the sizes differ.
    pub fn difference(&self, other: &Rgb) -> Option<Difference> {
        if (self.width, self.height) != (other.width, other.height) {
            return None;
        }
        let mut difference = Difference {
            pixels: 0,
            max_channel: 0,
        };
        // Branch-free so the compiler vectorises it: find compares thousands of frames.
        for (a, b) in self
            .pixels
            .as_chunks::<3>()
            .0
            .iter()
            .zip(other.pixels.as_chunks::<3>().0)
        {
            let gap = a[0]
                .abs_diff(b[0])
                .max(a[1].abs_diff(b[1]))
                .max(a[2].abs_diff(b[2]));
            difference.pixels += usize::from(gap != 0);
            difference.max_channel = difference.max_channel.max(gap);
        }
        Some(difference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_pictures_expand_like_the_windows_version() {
        let mut palette = Palette::BLACK;
        palette.0[1] = [63, 32, 0];
        let rgb = Rgb::from_indexed(&Image::new(2, 1, vec![1, 0]), &palette);
        assert_eq!(rgb.pixels, [252, 128, 0, 0, 0, 0]);
    }

    #[test]
    fn strips_put_frames_side_by_side() {
        let mut palette = Palette::BLACK;
        palette.0[1] = [63, 63, 63];
        let frames = [Image::new(1, 2, vec![1, 1]), Image::new(1, 1, vec![0])];
        let strip = Rgb::strip(&frames, &palette);
        assert_eq!((strip.width, strip.height), (2, 2));
        // The shorter second frame leaves black below it.
        assert_eq!(
            strip.pixels,
            [252, 252, 252, 0, 0, 0, 252, 252, 252, 0, 0, 0]
        );
    }

    #[test]
    fn png_files_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.png");
        let rgb = Rgb {
            width: 2,
            height: 1,
            pixels: vec![1, 2, 3, 250, 251, 252],
        };
        rgb.write_png(&path).unwrap();
        assert_eq!(Rgb::read_png(&path).unwrap(), rgb);
    }

    #[test]
    fn grey_and_palette_pngs_read_as_rgb() {
        // ImageMagick stores an all-grey screenshot (a black fade step, for one) as a greyscale
        // PNG, and may pick a palette PNG for few colours.
        let dir = tempfile::tempdir().unwrap();
        let grey = dir.path().join("grey.png");
        let mut encoder = png::Encoder::new(File::create(&grey).unwrap(), 2, 1);
        encoder.set_color(ColorType::Grayscale);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0, 200]).unwrap();
        writer.finish().unwrap();
        assert_eq!(
            Rgb::read_png(&grey).unwrap().pixels,
            [0, 0, 0, 200, 200, 200]
        );

        let indexed = dir.path().join("indexed.png");
        let mut encoder = png::Encoder::new(File::create(&indexed).unwrap(), 2, 1);
        encoder.set_color(ColorType::Indexed);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_palette(vec![10, 20, 30, 40, 50, 60]);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[1, 0]).unwrap();
        writer.finish().unwrap();
        assert_eq!(
            Rgb::read_png(&indexed).unwrap().pixels,
            [40, 50, 60, 10, 20, 30]
        );
    }

    #[test]
    fn differences_count_pixels_and_the_largest_channel_gap() {
        let a = Rgb {
            width: 2,
            height: 1,
            pixels: vec![0, 0, 0, 10, 10, 10],
        };
        let mut b = a.clone();
        b.pixels[4] = 14;
        assert_eq!(
            a.difference(&b),
            Some(Difference {
                pixels: 1,
                max_channel: 4
            })
        );
        assert_eq!(a.difference(&a).unwrap().pixels, 0);
        let tall = Rgb {
            width: 1,
            height: 2,
            pixels: a.pixels.clone(),
        };
        assert_eq!(a.difference(&tall), None);
    }
}
